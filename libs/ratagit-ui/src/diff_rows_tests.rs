use super::*;
use ratagit_core::partial::preview_rows;

#[test]
fn only_selection_gutter_is_added_to_native_diff_without_line_numbers() {
    let native = concat!(
        "diff --git a/a.rs b/a.rs\nindex 1111111..2222222 100644\n",
        "--- a/a.rs\n+++ b/a.rs\n@@ -1,4 +1,4 @@ function\n",
        " context\n-    old  \n+    new  \n \n",
        "\\ No newline at end of file\n",
        "@@ -50 +50,2 @@ another\n-old\n+new\n+@@ this is code\n",
    );
    let rows: Vec<_> = preview_rows(native)
        .into_iter()
        .map(|r| DiffRow::preview(r.row, r.staged))
        .collect();
    let lines = render(&rows, 0, 100);
    let content = lines
        .iter()
        .map(|line| line.spans[1].text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(format!("{content}\n"), native);
    assert!(
        lines
            .iter()
            .all(|line| line.spans.len() == 2 && line.spans[0].text == "   ")
    );
    assert_eq!(lines[6].text(), "   -    old  ");
    assert!(
        lines
            .iter()
            .any(|line| line.style.bg == Some(Color::Rgb(24, 46, 36)))
    );
    assert_eq!(lines[0].spans[1].style.fg, None);
    assert_eq!(lines[4].spans[1].style.fg, Some(current_theme().info));
    let code = lines
        .iter()
        .find(|line| line.spans[1].text == "+@@ this is code")
        .unwrap();
    assert_eq!(code.spans[1].style.fg, Some(Color::Green));
}

#[test]
fn tab_stops_are_relative_to_native_diff_content() {
    assert_eq!(expand_tabs("+\tword\tend"), "+       word    end");
    assert_eq!(expand_tabs("-  \tword"), "-       word");
    assert_eq!(expand_tabs(" 你好\tx"), " 你好   x");
}
