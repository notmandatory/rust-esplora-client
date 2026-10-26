#![cfg(target_arch = "wasm32")]

use core::future::Ready;
use core::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};

use esplora_client::{Builder, Sleeper};
use wasm_bindgen_test::wasm_bindgen_test;

fn base_url() -> &'static str {
    option_env!("ESPLORA_WASM_TEST_URL").expect("ESPLORA_WASM_TEST_URL must be set")
}

/// WASM has no default sleeper, so the tests provide their own.
struct NoopSleeper;

impl Sleeper for NoopSleeper {
    type Sleep = Ready<()>;

    fn sleep(_: Duration) -> Self::Sleep {
        core::future::ready(())
    }
}

static SLEEPS: AtomicUsize = AtomicUsize::new(0);

struct CountingSleeper;

impl Sleeper for CountingSleeper {
    type Sleep = Ready<()>;

    fn sleep(_: Duration) -> Self::Sleep {
        SLEEPS.fetch_add(1, Ordering::SeqCst);
        core::future::ready(())
    }
}

#[wasm_bindgen_test(async)]
async fn get_height() {
    let client = Builder::new(base_url())
        .build_async_with_sleeper::<NoopSleeper>()
        .expect("build client");

    assert_eq!(client.get_height().await.expect("get height"), 101);
}

#[wasm_bindgen_test(async)]
async fn get_fee_estimates() {
    let client = Builder::new(base_url())
        .build_async_with_sleeper::<NoopSleeper>()
        .expect("build client");

    let fees = client.get_fee_estimates().await.expect("get fee estimates");
    assert_eq!(fees.len(), 2);
    assert_eq!(fees.get(&1).expect("target 1").to_sat_per_kwu(), 375);
    assert_eq!(fees.get(&2).expect("target 2").to_sat_per_kwu(), 625);
}

#[wasm_bindgen_test(async)]
async fn retries_retryable_response() {
    SLEEPS.store(0, Ordering::SeqCst);

    let url = format!("{}/flaky", base_url());
    let client = Builder::new(&url)
        .max_retries(1)
        .build_async_with_sleeper::<CountingSleeper>()
        .expect("build client");

    assert_eq!(
        client.get_height().await.expect("get height after retry"),
        101
    );
    assert_eq!(SLEEPS.load(Ordering::SeqCst), 1);
}

#[wasm_bindgen_test(async)]
async fn applies_timeout() {
    let url = format!("{}/slow", base_url());
    let client = Builder::new(&url)
        .timeout(Duration::from_millis(100))
        .build_async_with_sleeper::<NoopSleeper>()
        .expect("build client");

    let error = client
        .get_height()
        .await
        .expect_err("slow request should time out");
    assert!(
        error.to_string().to_lowercase().contains("timeout"),
        "{error}"
    );
}
