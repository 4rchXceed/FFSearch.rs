#[cfg(feature = "ai")]
use ai_test::test_ai;
#[cfg(feature = "ai")]
mod ai_test;
#[cfg(feature = "mcp_server")]
mod mcp_test;

#[cfg(feature = "mcp_server")]
use crate::mcp_test::test_mcp;
use std::env;

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<String>>();
    let action = args.get(1).expect("An argument is required");

    if action == "ai" {
        #[cfg(feature = "ai")]
        test_ai().await;
    } else if action == "mcp" {
        #[cfg(feature = "mcp_server")]
        test_mcp().await;
    }
}
