use crate::time::Timestamp;
use crate::types::{Argument, FormattedString, UserEventChannel};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:[{channel}]='{formatted_string}'")]
pub struct UserEvent {
    pub timestamp: Timestamp,
    pub channel: UserEventChannel,
    pub formatted_string: FormattedString,
    pub args: Vec<Argument>,
}
