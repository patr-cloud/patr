use tokio::task::JoinHandle;

/// The resource executor task that will be used to manage the resources. This
/// will be used to spawn the tasks that will be used to manage the resources.
pub struct ResourceExecutorTask {
	/// The task that will be used to manage the resources.
	task: JoinHandle<()>,
}
