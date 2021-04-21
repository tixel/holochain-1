use tempfile;

use crate::*;

pub(crate) struct WorkspaceMocker {
    dir: tempfile::TempDir,
}

impl WorkspaceMocker {
    pub(crate) fn try_new() -> Fallible<Self> {
        todo!("");
    }
}
