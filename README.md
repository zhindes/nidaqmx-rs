# nidaqmx-rs: DAQmx Rust Bindings Proof of Concept

This is a proof of concept for two types of NI-DAQmx bindings in Rust.

> Note: Currently this project has only been built and tested on Windows. Linux should work just the
same, but I didn't try it out. At minimum, it would be required to add the Linux NI-DAQmx lib to
[ext/](ext/) and update [build.rs](build.rs) to set up the link paths correctly per-platform.

## nidaqmx_sys: Raw NI-DAQmx C Bindings

This API is created using [bindgen](https://github.com/rust-lang/rust-bindgen) (documentation
[here](https://rust-lang.github.io/rust-bindgen/)). At build-time, bindgen automatically generates
bindings to the C API using the NIDAQmx.h header file (checked into this repository under the
[ext/](ext/) folder) and links to the library (also in [ext/](ext/)).

By necessity, these bindings are `unsafe`, but serve as a core building block that could be used
directly or wrapped in a more Rust-like API. A few examples of what these bindings look like:

```Rust
extern "C" {
    pub fn DAQmxCreateTask(
        taskName: *const ::std::os::raw::c_char,
        taskHandle: *mut TaskHandle,
    ) -> int32;
}

// Example constant for terminal config
// !!! Notice that the type here doesn't match the function signature, sadly.
pub const DAQmx_Val_Diff: u32 = 10106;
// Example constant for units
// !!! Notice that the type here doesn't match the function signature, sadly.
pub const DAQmx_Val_FromCustomScale: u32 = 10065;
// Example error code that can be returned
pub const DAQmxErrorChanAlreadyInTask: i32 = -200489;

extern "C" {
    pub fn DAQmxCreateAIVoltageChan(
        taskHandle: TaskHandle,
        physicalChannel: *const ::std::os::raw::c_char,
        nameToAssignToChannel: *const ::std::os::raw::c_char,
        terminalConfig: int32,
        minVal: float64,
        maxVal: float64,
        units: int32,
        customScaleName: *const ::std::os::raw::c_char,
    ) -> int32;
}
```

There is a test that uses these bindings in [src/nidaqmx_sys.rs](src/nidaqmx_sys.rs) which
demonstrates how to use it.

```Rust
unsafe {
    // Create a task.
    let taskName = CString::new("lib_create_close_task").unwrap();
    let mut task: TaskHandle = mem::zeroed();
    let mut result = DAQmxCreateTask(taskName.as_ptr(), &mut task);
    assert_eq!(result, 0);

    // Create a duplicate task with the same name; this should error.
    {
        let mut dupeTask: TaskHandle = mem::zeroed();
        let dupeResult = DAQmxCreateTask(taskName.as_ptr(), &mut dupeTask);
        assert_eq!(dupeResult, DAQmxErrorDuplicateTask);
    }

    // Clear the task when we're done.
    result = DAQmxClearTask(task);
    assert_eq!(result, 0);
}
```

Using bindgen for this is very straight-forward and allows you to quickly use NI-DAQmx in Rust.
There are two issues with this approach:
1. **The automatic conversion isn't quite right** - For example, as was noted above, enumeration
types don't match the associated parameters in function signatures. A better approach might be to
code-generate these bindings using NI-DAQmx API metadata rather than leveraging bindgen.
Unfortunately, the NI-DAQmx API metadata is not publicly available.
2. **The API isn't very Rust-like** - Raw `unsafe` function wrappers using C types is a big chore to
use in a proper Rust program. That is why creating a richer abstraction is desirable. The next
proof-of-concept demonstrates that.

## nidaqmx_rs: Proper Rust Bindings for NI-DAQmx

Using **nidaqmx_sys**, one could build proper Rust bindings that provide the safety guarantees that
make Rust so powerful. A small prototype has been provided here and is implemented in
[src/lib.rs](src/lib.rs). The basic design has been lifted directly from
[nidaqmx-python](https://github.com/ni/nidaqmx-python), the NI-DAQmx Python API. That API provides a
rich object-oriented API for using NI-DAQmx. This is very appropriate for Rust as well. A brief
overview of what has been provided so far:
* **Error** struct - Functions that can return an NI-DAQmx error return a `Result` using the `Error`
struct.
* **Task** struct - Owns a task handle and implements the `Drop` trait to handle task lifetime.
* **AIChannel** struct - Represents one or more Analog Input Channels.
* **AIChannelCollection** struct - Represents all of the channels in an Analog Input task.

With this core implementation, you can create a task and add voltage channels it. The implementation
has a few tests that demonstrate how to use it (they require a real or simulated DAQ device to
execute). Here is an example:

```Rust
// Create a task.
let task = Task::new("my_task";).unwrap();
// Create a voltage channel.
let ai0 = task.ai_channels.add_ai_voltage_chan("Dev1/ai0", -5.0, 5.0).unwrap();

// Create a duplicate voltage channel; this should error.
let err = match task.ai_channels.add_ai_voltage_chan("Dev1/ai0", -5.0, 5.0) {
    // Not erroring isn't expected.
    Ok(_) => panic!("duplicate virtual channel name should error"),
    Err(err) => err,
};
assert_eq!(err.code, nidaqmx_sys::DAQmxErrorChanAlreadyInTask);

// Create a voltage channel.
task.ai_channels.add_ai_voltage_chan("Dev1/ai1", -5.0, 5.0).unwrap();

// Create an AIChannel object for the second channel in the task.
let ai1 = task.ai_channels.channel_at(1).unwrap();
assert_eq!(ai0.name, "Dev1/ai1");
```

The implementation is partial at-best, but shows off how you could build the rest of the API using
these same concepts. In the future, this could be completely code-generated using metadata that
describes the NI-DAQmx API. Unfortunately, the NI-DAQmx API metadata is not publicly available. In
the short-term, users could add the features they require manually over time.
