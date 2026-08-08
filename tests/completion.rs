use teleminator::completion::*;

#[test]
fn completes_filter_prefix() {
    let items = command_suggestions("fil", 0);
    assert!(items.iter().any(|s| s.text == "filter"));
    assert!(
        items.iter().all(|s| {
            s.text != "filter-in" && s.text != "filter-out" && s.text != "filters"
        })
    );
}

#[test]
fn suggestions_sorted_alphanumerically() {
    let mut items = command_suggestions("c", 0);
    sort_suggestions(&mut items);
    let texts: Vec<&str> = items.iter().map(|s| s.text.as_str()).collect();
    assert!(
        texts
            .windows(2)
            .all(|w| { w[0].to_ascii_lowercase() <= w[1].to_ascii_lowercase() })
    );
    assert!(texts.contains(&"copy"));
    assert!(texts.contains(&"config"));
    let copy = texts.iter().position(|t| *t == "copy").unwrap();
    let config = texts.iter().position(|t| *t == "config").unwrap();
    assert!(config < copy);
}

#[test]
fn completes_filter_subcommands() {
    let items = filter_suggestions("", 0, 0);
    assert!(items.iter().any(|s| s.text == "list"));
    assert!(items.iter().any(|s| s.text == "in"));
    assert!(items.iter().any(|s| s.text == "out"));
    assert!(items.iter().any(|s| s.text == "toggle"));
    assert!(items.iter().any(|s| s.text == "set"));
    assert!(items.iter().any(|s| s.text == "clear"));
    assert!(items.iter().any(|s| s.text == "delete"));
    let set = filter_suggestions("set ", 4, 2);
    assert!(set.iter().any(|s| s.text == "toggle"));
    assert!(set.iter().any(|s| s.text == "on"));
}

#[test]
fn does_not_suggest_keybinding_only_commands() {
    let items = command_suggestions("", 0);
    assert!(items.iter().any(|s| s.text == "quit"));
    assert!(items.iter().any(|s| s.text == "hide"));
    assert!(items.iter().any(|s| s.text == "delete"));
    assert!(items.iter().any(|s| s.text == "config"));
    assert!(items.iter().all(|s| s.text != "theme"));
    assert!(items.iter().all(|s| {
        !matches!(
            s.text.as_str(),
            "nav"
                | "page"
                | "scroll"
                | "match"
                | "focus"
                | "search"
                | "command"
                | "q"
                | "d"
                | "D"
        )
    }));
}

#[test]
fn completes_config_set_get() {
    let items = config_suggestions("", 0);
    assert!(items.iter().any(|s| s.text == "set"));
    assert!(items.iter().any(|s| s.text == "get"));
    let keys = config_suggestions("get ", 4);
    assert!(keys.iter().any(|s| s.text == "view.main.tail_mode"));
    assert!(keys.iter().any(|s| s.text == "line_numbers.enabled"));
}

#[test]
fn common_prefix_works() {
    assert_eq!(
        common_prefix(["filter", "follow", "fold"].into_iter()).as_deref(),
        Some("f")
    );
    assert_eq!(
        common_prefix(["list", "in", "out"].into_iter()).as_deref(),
        None
    );
    assert_eq!(
        common_prefix(["filter-in", "filter-out"].into_iter()).as_deref(),
        Some("filter-")
    );
}

#[test]
fn selection_starts_unselected_and_cycles() {
    let mut state = CompletionState::default();
    assert!(state.selected.is_none());
    state.items = vec![
        Suggestion {
            text: "a".into(),
            label: "a".into(),
            help: String::new(),
            replace_from: 0,
        },
        Suggestion {
            text: "b".into(),
            label: "b".into(),
            help: String::new(),
            replace_from: 0,
        },
    ];
    state.select_next();
    assert_eq!(state.selected, Some(0));
    state.select_next();
    assert_eq!(state.selected, Some(1));
    state.select_next();
    assert_eq!(state.selected, Some(0));
}
