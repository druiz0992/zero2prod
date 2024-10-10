mod health_check;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;
mod unsubscribe;

pub use health_check::*;
pub use newsletters::*;
pub(crate) use subscriptions::get_token_from_subscriber_id;
pub use subscriptions::subscribe;
pub use subscriptions_confirm::confirm;
pub use unsubscribe::unsubscribe;

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
