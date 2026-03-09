use crate::env;
use crate::test_support::env_lock;

#[test]
fn check_python_returns_env_item() {
    let _guard = env_lock();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let item = rt.block_on(env::check_python());
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

#[test]
fn check_uv_returns_env_item() {
    let _guard = env_lock();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let item = rt.block_on(env::check_uv());
    assert_eq!(item.name, "uv");
    if item.found {
        assert!(item.path.is_some());
        assert!(item.version.is_some());
    } else {
        assert!(item.suggestion.is_some());
    }
}

#[test]
fn check_rust_returns_env_item() {
    let _guard = env_lock();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let item = rt.block_on(env::check_rust());
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

#[test]
fn env_items_have_correct_names() {
    let _guard = env_lock();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let python = rt.block_on(env::check_python());
    let uv = rt.block_on(env::check_uv());
    let rust = rt.block_on(env::check_rust());

    assert_eq!(python.name, "Python");
    assert_eq!(uv.name, "uv");
    assert_eq!(rust.name, "Rust");
}

#[test]
fn env_item_found_implies_path() {
    let _guard = env_lock();
    let rt = tokio::runtime::Runtime::new().unwrap();
    // If an item is found, it must have a path
    let items = vec![
        rt.block_on(env::check_python()),
        rt.block_on(env::check_uv()),
        rt.block_on(env::check_rust()),
    ];
    for item in items {
        if item.found {
            assert!(item.path.is_some(), "{} found but has no path", item.name);
        }
    }
}
