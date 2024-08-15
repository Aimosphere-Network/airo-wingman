#[macro_export]
macro_rules! retry_on_err_or_none {
    ($n:expr, $interval:expr, $fn:expr) => {{
        let mut retries = 0;
        loop {
            match $fn {
                Ok(None) | Err(_) if retries < $n => retries += 1,
                res => break res,
            }
            tokio::time::sleep(core::time::Duration::from_millis($interval)).await;
        }
    }};
    ($n:expr, $fn:expr) => {
        retry_on_err_or_none!($n, 1000, $fn)
    };
}
