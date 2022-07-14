#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CString};
    use std::mem;

    #[test]
    fn create_close_task() {
        unsafe {
            let taskName = CString::new("lib_create_close_task").unwrap();
            let mut task: TaskHandle = mem::zeroed();
            let mut result = DAQmxCreateTask(taskName.as_ptr(), &mut task);
            assert_eq!(result, 0);

            // Validate a simple error case.
            {
                let mut dupeTask: TaskHandle = mem::zeroed();
                let dupeResult = DAQmxCreateTask(taskName.as_ptr(), &mut dupeTask);
                assert_eq!(dupeResult, DAQmxErrorDuplicateTask);
            }

            result = DAQmxClearTask(task);
            assert_eq!(result, 0);
        }
    }
}
