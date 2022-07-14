#![allow(dead_code)]

mod nidaqmx_sys;

use std::cmp::Ordering;
use std::ffi::CString;
use std::mem;

use nidaqmx_sys::TaskHandle;
use nidaqmx_sys::int32;
        
// todo: How to handle warnings?
#[derive(Debug)]
struct Error {
    code: i32,
    description: String,
    extended_info: String
}

struct Task {
    handle: TaskHandle,
    ai_channels: AIChannelCollection
}

struct AIChannel {
    task_handle: TaskHandle,
    name: String
}

struct AIChannelCollection {
    task_handle: TaskHandle
}

impl Task {
    fn new(name: &str) -> Result<Task, Error> {
        // todo: propagate this error, too.
        let name_cstr = CString::new(name).unwrap();
        let mut task: TaskHandle;
        let ret_code: int32;
        unsafe {
            task = mem::zeroed();
            ret_code = nidaqmx_sys::DAQmxCreateTask(name_cstr.as_ptr(), &mut task);
        }
        // todo: nidaqmx_sys::DAQmxSuccess has the wrong type.
        match ret_code.cmp(&0) {
            Ordering::Less =>
                Err(Error {
                        code: ret_code,
                        // todo: handle these
                        description: String::new(),
                        extended_info: String::new()
                    }
                ),
            _ => Ok(Task {
                        handle: task,
                        ai_channels: AIChannelCollection{task_handle: task}
                    }
                )
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // todo: ignore ret code?
        unsafe {
            nidaqmx_sys::DAQmxClearTask(self.handle);
            self.handle = mem::zeroed();
        }
    }
}

impl AIChannelCollection {
    fn add_ai_voltage_chan(
        &self,
        physical_channel: &str,
        min_val: f64,
        max_val: f64) -> Result<AIChannel, Error> {
        // todo: propagate this error, too.
        let phys_name_cstr = CString::new(physical_channel).unwrap();
        // todo: this should be parameterized, but then the AIChannel name has to match this if non-empty.
        let virt_name_cstr = CString::new("").unwrap();
        // todo: scale name is useless without units
        let scale_name_cstr = CString::new("").unwrap();
        let ret_code: int32;
        unsafe {
            ret_code = nidaqmx_sys::DAQmxCreateAIVoltageChan(
                self.task_handle,
                phys_name_cstr.as_ptr(),
                virt_name_cstr.as_ptr(),
                nidaqmx_sys::DAQmx_Val_Default,
                min_val,
                max_val,
                // todo: first-class enums
                nidaqmx_sys::DAQmx_Val_Volts.try_into().unwrap(),
                scale_name_cstr.as_ptr()
            );
        }
        // todo: nidaqmx_sys::DAQmxSuccess has the wrong type.
        match ret_code.cmp(&0) {
            Ordering::Less =>
                Err(Error {
                        code: ret_code,
                        // todo: handle these
                        description: String::new(),
                        extended_info: String::new()
                    }
                ),
            _ => Ok(AIChannel {
                        task_handle: self.task_handle,
                        name: physical_channel.to_string()
                    }
                )
        }
    }

    fn channel_names(&self) -> Result<Vec<String>, Error> {
        let mut buf_size: usize = 0;
        let channel_names: String;

        let mut ret_code: int32;
        loop {
            let mut bytes: Vec<u8> = Vec::new();
            bytes.resize(buf_size, 10 /* newline */);
            let mut c_string: CString;

            // todo: handle error
            c_string = CString::new(bytes).unwrap();
            let raw_c_string = c_string.into_raw();
            unsafe {
                ret_code = nidaqmx_sys::DAQmxGetTaskChannels(
                    self.task_handle,
                    raw_c_string,
                    buf_size.try_into().unwrap(),
                );
                c_string = CString::from_raw(raw_c_string);
            }
            if (ret_code == nidaqmx_sys::DAQmxErrorBufferTooSmallForString) || (ret_code == nidaqmx_sys::DAQmxWarningCAPIStringTruncatedToFitBuffer.try_into().unwrap()) {
                buf_size = 0;
            } else if (ret_code > 0) && (buf_size == 0) {
                buf_size = ret_code.try_into().unwrap();
            } else {
                // todo: handle_error
                channel_names = c_string.into_string().unwrap();
                break;
            }
        }
        match ret_code.cmp(&0) {
            Ordering::Less =>
                Err(Error {
                        code: ret_code,
                        // todo: handle these
                        description: String::new(),
                        extended_info: String::new()
                    }
                ),
            _ => Ok(unflatten_channel_string(&channel_names))
        }
    }

    // We can't implement the Index trait because that requires returning references, but we make
    // objects on-demand. this allows for random sets of channel to be referenced collectively at
    // run-time.
    fn channel_at(&self, index: usize) -> Result<AIChannel, Error> {
        match self.channel_names() {
            Ok(names) => Ok(AIChannel{task_handle: self.task_handle, name: names[index].clone()}),
            Err(err) => Err(err)
        }
    }

    /* todo:
    use std::ops::Range;
    fn channel_at(&self, index: Range<usize>) -> Result<AIChannel, Error> {}
    fn channel_at(&self, index: RangeFrom<usize>) -> Result<AIChannel, Error> {}
    fn channel_at(&self, index: RangeFull<usize>) -> Result<AIChannel, Error> {}
    fn channel_at(&self, index: RangeInclusive<usize>) -> Result<AIChannel, Error> {}
    fn channel_at(&self, index: RangeTo<usize>) -> Result<AIChannel, Error> {}
    fn channel_at(&self, index: RangeToInclusive<usize>) -> Result<AIChannel, Error> {}
    */
}

// todo: move to utils submodule
fn unflatten_channel_string(channel_names: &str) -> Vec<String> {
    // todo: this isn't the full impl, gotta handle colons, etc.
    channel_names.split(",").map(|s| s.trim().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unflatten_channel_string() {
        let chans = "Dev1/ai0, Dev1/ai1";
        let chans_unflat = unflatten_channel_string(chans);
        assert_eq!(chans_unflat.len(), 2);
        assert_eq!(chans_unflat[0], "Dev1/ai0");
        assert_eq!(chans_unflat[1], "Dev1/ai1");
    }

    #[test]
    fn create_close_task() {
        let task_name = "create_close_task";
        {
            let _task = Task::new(task_name).unwrap();
            let err = match Task::new(task_name) {
                Ok(_) => panic!("duplicate task should error"),
                Err(err) => err,
            };
            assert_eq!(err.code, nidaqmx_sys::DAQmxErrorDuplicateTask);
        }
        // original task should be dropped, so we can reuse the name.
        {
            if let Err(_) = Task::new(task_name) {
                panic!("Task name should be available");
            }
        }
    }

    #[test]
    fn create_ai_chan() {
        let task_name = "create_ai_chan";
        {
            let task = Task::new(task_name).unwrap();
            task.ai_channels.add_ai_voltage_chan("Dev1/ai0", -5.0, 5.0).unwrap();
            let err = match task.ai_channels.add_ai_voltage_chan("Dev1/ai0", -5.0, 5.0) {
                Ok(_) => panic!("duplicate virtual channel name should error"),
                Err(err) => err,
            };
            assert_eq!(err.code, nidaqmx_sys::DAQmxErrorChanAlreadyInTask);
        }
    }

    #[test]
    fn ai_channel_names() {
        let task_name = "ai_channel_names";
        {
            let task = Task::new(task_name).unwrap();
            task.ai_channels.add_ai_voltage_chan("Dev1/ai0", -5.0, 5.0).unwrap();
            task.ai_channels.add_ai_voltage_chan("Dev1/ai1", -5.0, 5.0).unwrap();

            let channel_names = task.ai_channels.channel_names().unwrap();
            assert_eq!(channel_names.len(), 2);
            assert_eq!(channel_names[0], "Dev1/ai0");
            assert_eq!(channel_names[1], "Dev1/ai1");
        }
    }

    #[test]
    fn ai_channel_indexing() {
        let task_name = "ai_channel_indexing";
        {
            let task = Task::new(task_name).unwrap();
            task.ai_channels.add_ai_voltage_chan("Dev1/ai0", -5.0, 5.0).unwrap();
            task.ai_channels.add_ai_voltage_chan("Dev1/ai1", -5.0, 5.0).unwrap();

            let ai0 = task.ai_channels.channel_at(0).unwrap();
            assert_eq!(ai0.name, "Dev1/ai0");
            let ai1 = task.ai_channels.channel_at(1).unwrap();
            assert_eq!(ai1.name, "Dev1/ai1");
        }
    }

    #[test]
    #[should_panic]
    fn ai_invalid_channel_indexing() {
        let task_name = "ai_invalid_channel_indexing";
        {
            let task = Task::new(task_name).unwrap();
            task.ai_channels.add_ai_voltage_chan("Dev1/ai0", -5.0, 5.0).unwrap();
            task.ai_channels.add_ai_voltage_chan("Dev1/ai1", -5.0, 5.0).unwrap();
            // index out of bounds should_panic, so match everything successfully
            match task.ai_channels.channel_at(2) {
                _ => ()
            };
        }
    }

}