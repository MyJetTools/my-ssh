use std::time::Duration;

pub async fn await_would_block<TResult>(
    mut execute: impl FnMut() -> Result<TResult, ssh2::Error>,
) -> Result<TResult, ssh2::Error> {
    loop {
        let result = execute();

        match result {
            Ok(result) => return Ok(result),
            Err(e) => match e.code() {
                ssh2::ErrorCode::Session(code) => {
                    if code == -37 {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                }
                _ => return Err(e),
            },
        }
    }
}
