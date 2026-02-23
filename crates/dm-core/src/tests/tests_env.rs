use crate::env;

#[tokio::test]
async fn check_python_returns_env_item() {
    let item = env::check_python().await;
    assert_eq!(item.name, "Python");
    // On this machine Python should be found
    if item.found {
        assert!(item.path.is_some());
        assert!(item.version.is_some());
        assert!(item.suggestion.is_none());
    } else {
        // If not found, suggestion should be present
        assert!(item.suggestion.is_some());
        assert!(item.path.is_none());
    }
}

#[tokio::test]
async fn check_uv_returns_env_item() {
    let item = env::check_uv().await;
    assert_eq!(item.name, "uv");
    if item.found {
        assert!(item.path.is_some());
        assert!(item.version.is_some());
    } else {
        assert!(item.suggestion.is_some());
    }
}

#[tokio::test]
async fn check_rust_returns_env_item() {
    let item = env::check_rust().await;
    assert_eq!(item.name, "Rust");
    if item.found {
        assert!(item.path.is_some());
        assert!(item.version.is_some());
        // Version should contain "cargo"
        assert!(item.version.as_ref().unwrap().contains("cargo"));
    } else {
        assert!(item.suggestion.is_some());
    }
}

#[tokio::test]
async fn env_items_have_correct_names() {
    let python = env::check_python().await;
    let uv = env::check_uv().await;
    let rust = env::check_rust().await;

    assert_eq!(python.name, "Python");
    assert_eq!(uv.name, "uv");
    assert_eq!(rust.name, "Rust");
}

#[tokio::test]
async fn env_item_found_implies_path() {
    // If an item is found, it must have a path
    let items = vec![
        env::check_python().await,
        env::check_uv().await,
        env::check_rust().await,
    ];
    for item in items {
        if item.found {
            assert!(item.path.is_some(), "{} found but has no path", item.name);
        }
    }
}
