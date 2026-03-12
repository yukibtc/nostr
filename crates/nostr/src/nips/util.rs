use crate::types::url::{self, RelayUrl};

pub(super) fn take_and_parse_relay_hint<T, S>(iter: &mut T) -> Result<Option<RelayUrl>, url::Error>
where
    T: Iterator<Item = S>,
    S: AsRef<str>,
{
    match iter.next() {
        Some(url) => {
            let url: &str = url.as_ref();

            if url.is_empty() {
                Ok(None)
            } else {
                Ok(Some(RelayUrl::parse(url)?))
            }
        }
        None => Ok(None),
    }
}
