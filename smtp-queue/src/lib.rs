pub mod execution;
pub mod queue;

// Keep existing crate paths stable while organizing implementation by feature.
pub use execution::{processor, worker};
pub use queue::{config, error, model, retry, store};

pub use execution::{
    DeliveryBatchProcessor, DeliveryProcessingError, Processor, TestDeliveryBatchProcessor,
    TestProcessor, TestProcessorOutcome, Worker,
};
pub use queue::{
    ClaimedDeliveryBatch, ClaimedRecipient, DeadLetterItem, DeadLetterRecipient, EnqueueRequest,
    EnqueueResult, FailureCategory, MessageId, Queue, QueueConfig, QueueError, QueueState,
    RecipientDeliveryResult, RecipientDeliveryState,
};
