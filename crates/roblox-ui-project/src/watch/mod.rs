/*!
    Recursive, debounced filesystem watcher used by the background sync loop.
*/

mod watcher;

pub use watcher::AsyncFileWatcher;
