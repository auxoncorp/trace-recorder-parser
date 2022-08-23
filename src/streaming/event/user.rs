use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{Argument, FormatString, FormattedString, UserEventChannel};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:[{channel}]='{formatted_string}'")]
pub struct UserEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub channel: UserEventChannel,
    pub format_string: FormatString,
    pub formatted_string: FormattedString,
    pub args: Vec<Argument>,
}
