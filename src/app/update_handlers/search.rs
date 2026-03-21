use crate::app::{App, Command, Message};

pub(crate) fn handle_search_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::StartSearchInput => {
            app.start_search_input();
            app.push_log("search: type query, Enter confirm, Esc cancel", true);
            app.dirty.mark();
        }
        Message::SearchSetQuery(query) => {
            let count = app.apply_search_query(query);
            if count > 0 {
                app.search_select_initial_match();
            }
            app.reload_diff_now();
            app.dirty.mark();
        }
        Message::SearchConfirm => {
            let count = app.apply_search_query(app.search_query.clone());
            if count == 0 {
                app.push_log(format!("search no match: {}", app.search_query), false);
            } else {
                app.push_log(
                    format!("search match {}: {}", count, app.search_query),
                    true,
                );
            }
            app.reload_diff_now();
            app.dirty.mark();
        }
        Message::SearchClear => {
            app.clear_search();
            app.cancel_input();
            app.push_log("search cleared", true);
            app.reload_diff_now();
            app.dirty.mark();
        }
        Message::SearchNext => {
            if app.search_jump_next() {
                app.reload_diff_now();
                app.dirty.mark();
            }
        }
        Message::SearchPrev => {
            if app.search_jump_prev() {
                app.reload_diff_now();
                app.dirty.mark();
            }
        }
        _ => {}
    }
    None
}
