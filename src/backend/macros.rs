/// Generates CommandHandler implementations from declarative definitions
///
/// Syntax:
/// ```ignore
/// define_handlers! {
///     CommandVariant => git_ops_function => SuccessEvent,
///     CommandVariant { field } => git_ops_function(&field) => SuccessEvent(result),
///     CommandVariant [mut] => git_ops_function => SuccessEvent,
/// }
/// ```
#[macro_export]
macro_rules! define_handlers {
    ($($tokens:tt)*) => {
        $crate::__define_handlers_impl!($($tokens)*);
    };
}

// Internal implementation macro
#[doc(hidden)]
#[macro_export]
macro_rules! __define_handlers_impl {
    // Entry point: process all handler definitions
    (
        $(
            $cmd_variant:ident $({ $($field:ident),* })? $([ $mut_flag:ident ])? =>
            $git_fn:expr =>
            $event_variant:ident $( ( $result_binding:ident ) )?
        ),* $(,)?
    ) => {
        // Generate handler structs
        $(
            $crate::__generate_handler_struct!($cmd_variant);
        )*

        // Generate CommandHandler trait implementations
        $(
            $crate::__generate_handler_impl!(
                $cmd_variant $({ $($field),* })? $([ $mut_flag ])? =>
                $git_fn =>
                $event_variant $( ( $result_binding ) )?
            );
        )*

        // Generate handler registry
        $crate::__generate_registry!($($cmd_variant),*);
    };
}

// Generate handler struct
#[doc(hidden)]
#[macro_export]
macro_rules! __generate_handler_struct {
    ($cmd_variant:ident) => {
        paste::paste! {
            pub struct [<$cmd_variant Handler>];
        }
    };
}

// Generate CommandHandler trait implementation
#[doc(hidden)]
#[macro_export]
macro_rules! __generate_handler_impl {
    // Simple command with no fields, immutable repo
    (
        $cmd_variant:ident =>
        $git_fn:expr =>
        $event_variant:ident
    ) => {
        paste::paste! {
            impl CommandHandler for [<$cmd_variant Handler>] {
                fn handle(
                    &self,
                    repo: &GitRepo,
                    request_id: u64,
                ) -> Result<Vec<EventEnvelope>> {
                    let result = $git_fn(repo)?;
                    Ok(vec![EventEnvelope {
                        request_id: Some(request_id),
                        event: FrontendEvent::$event_variant,
                    }])
                }
            }
        }
    };

    // Command with fields, immutable repo
    (
        $cmd_variant:ident { $($field:ident),* } =>
        $git_fn:expr =>
        $event_variant:ident ( $result_binding:ident )
    ) => {
        paste::paste! {
            impl CommandHandler for [<$cmd_variant Handler>] {
                fn handle(
                    &self,
                    repo: &GitRepo,
                    request_id: u64,
                ) -> Result<Vec<EventEnvelope>> {
                    let $result_binding = $git_fn(repo)?;
                    Ok(vec![EventEnvelope {
                        request_id: Some(request_id),
                        event: FrontendEvent::$event_variant($result_binding),
                    }])
                }
            }
        }
    };

    // Command with mutable repo flag
    (
        $cmd_variant:ident [ mut ] =>
        $git_fn:expr =>
        $event_variant:ident ( $result_binding:ident )
    ) => {
        paste::paste! {
            impl CommandHandler for [<$cmd_variant Handler>] {
                fn handle_mut(
                    &self,
                    repo: &mut GitRepo,
                    request_id: u64,
                ) -> Result<Vec<EventEnvelope>> {
                    let $result_binding = $git_fn(repo)?;
                    Ok(vec![EventEnvelope {
                        request_id: Some(request_id),
                        event: FrontendEvent::$event_variant($result_binding),
                    }])
                }
            }
        }
    };
}

// Generate handler registry (stub for Task 1.3)
#[doc(hidden)]
#[macro_export]
macro_rules! __generate_registry {
    ($($cmd_variant:ident),* $(,)?) => {
        // Stub: will be implemented in Task 1.3
    };
}
