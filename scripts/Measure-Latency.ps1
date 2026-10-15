[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Path = 'ratagit-latency.jsonl'
)

$ErrorActionPreference = 'Stop'
$latencySamples = [System.Collections.Generic.List[object]]::new()
$latencyPublications = @{}
$latencyUnobserved = 0

foreach ($latencyLine in [System.IO.File]::ReadLines((Resolve-Path -LiteralPath $Path).Path)) {
    if ([string]::IsNullOrWhiteSpace($latencyLine)) { continue }
    $latencyEvent = $latencyLine | ConvertFrom-Json
    if ($latencyEvent.schema -ne 1) { throw 'Unsupported latency event schema' }
    $latencyKey = '{0}/{1}/{2}' -f $latencyEvent.request_id, $latencyEvent.span_id, $latencyEvent.state_version
    if ($latencyEvent.stage -eq 'snapshot_published') {
        $latencyPublications[$latencyKey] = [double]$latencyEvent.elapsed_us
        continue
    }
    if ($latencyEvent.stage -in @('snapshot_superseded', 'frame_unobserved', 'draw_failed')) {
        $latencyUnobserved++
        $latencyPublications.Remove($latencyKey)
        continue
    }
    $latencyMetrics = @{}
    if ($latencyEvent.stage -eq 'frame_drawn') {
        $latencyMetric = if ($null -eq $latencyEvent.parent_span_id) { 'request_to_first_frame' } else { 'request_to_result_frame' }
        $latencyMetrics[$latencyMetric] = [double]$latencyEvent.elapsed_us
        $latencyMetrics['draw'] = [double]$latencyEvent.duration_us
        if ($latencyPublications.ContainsKey($latencyKey)) {
            $latencyMetrics['publication_to_frame'] = [double]$latencyEvent.elapsed_us - $latencyPublications[$latencyKey]
            $latencyPublications.Remove($latencyKey)
        }
    } else {
        $latencyMetric = switch ($latencyEvent.stage) {
            'action_dequeued' { 'action_queue' }
            'command_dequeued' { 'command_queue' }
            'effect_started' { 'effect_queue' }
            'git_started' { 'git_queue' }
            'git_finished' { 'git_execution' }
            'dispatch_finished' { 'reduce' }
            'snapshot_built' { 'snapshot' }
        }
        if ($latencyMetric) { $latencyMetrics[$latencyMetric] = [double]$latencyEvent.duration_us }
    }
    foreach ($latencyMetric in $latencyMetrics.Keys) {
        $latencySamples.Add([pscustomobject]@{
            Metric = $latencyMetric
            Origin = $latencyEvent.origin
            Operation = $latencyEvent.operation
            Microseconds = $latencyMetrics[$latencyMetric]
        })
    }
}

Write-Verbose "Unobserved publications excluded from frame metrics: $latencyUnobserved"
$latencySamples | Group-Object Metric, Origin, Operation | ForEach-Object {
    $latencyValues = @($_.Group.Microseconds | Sort-Object)
    $latencyCount = $latencyValues.Count
    $latencyMiddle = [int][math]::Floor($latencyCount / 2)
    $latencyMedian = if ($latencyCount % 2) { $latencyValues[$latencyMiddle] } else {
        ($latencyValues[$latencyMiddle - 1] + $latencyValues[$latencyMiddle]) / 2
    }
    [pscustomobject]@{
        Metric = $_.Group[0].Metric
        Origin = $_.Group[0].Origin
        Operation = $_.Group[0].Operation
        Samples = $latencyCount
        MedianMs = [math]::Round($latencyMedian / 1000, 3)
        P95Ms = [math]::Round($latencyValues[[int][math]::Ceiling(0.95 * $latencyCount) - 1] / 1000, 3)
        MaxMs = [math]::Round($latencyValues[-1] / 1000, 3)
    }
} | Sort-Object Metric, Origin, Operation
