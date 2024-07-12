use pretty_assertions::assert_eq;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use trace_recorder_parser::{streaming::event::*, streaming::*, time::*, types::*};

const TRACE_V10: &str = "test_resources/fixtures/streaming/v10/trace.psf";
const TRACE_V12: &str = "test_resources/fixtures/streaming/v12/trace.psf";
const TRACE_V13: &str = "test_resources/fixtures/streaming/v13/trace.psf";
const TRACE_V14: &str = "test_resources/fixtures/streaming/v14/trace.psf";

fn open_trace_file(trace_path: &str) -> File {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(trace_path);
    File::open(path).unwrap()
}

struct TestRecorderData<R: Read> {
    rd: RecorderData,
    f: R,
    event_cnt: u16,
    timestamp_ticks: u64,
}

impl<R> TestRecorderData<R>
where
    R: Read,
{
    pub fn check_event(&mut self, typ: EventType) {
        let (ec, ev) = self.rd.read_event(&mut self.f).unwrap().unwrap();
        assert_eq!(ec.event_type(), typ);
        assert_eq!(u16::from(ev.event_count()), self.event_cnt);
        assert_eq!(ev.timestamp().ticks(), self.timestamp_ticks);

        self.event_cnt += 1;
        self.timestamp_ticks += 1;
    }
}

// git tag: Tz4/4.6/v4.6.6
#[test]
fn streaming_v10_smoke() {
    common_tests(CommonTestConfig {
        trace_path: TRACE_V10,
        expected_trace_format_version: 10,
        expected_platform_cfg_version_minor: 0,
        initial_event_count: 1,
        latest_timestamp: Timestamp::zero(),
        high_water_mark: 0,
        extra_user_events: false,
    });
}

// git tag: Tz4/4.7/v4.7.0
#[test]
fn streaming_v12_smoke() {
    common_tests(CommonTestConfig {
        trace_path: TRACE_V12,
        expected_trace_format_version: 12,
        expected_platform_cfg_version_minor: 2,
        initial_event_count: 6,
        latest_timestamp: Timestamp::zero(),
        high_water_mark: 0,
        extra_user_events: false,
    });
}

// git tag: Tz4/4.8/v4.8.0.hotfix1
#[test]
fn streaming_v13_smoke() {
    common_tests(CommonTestConfig {
        trace_path: TRACE_V13,
        expected_trace_format_version: 13,
        expected_platform_cfg_version_minor: 2,
        initial_event_count: 6,
        latest_timestamp: Timestamp::zero(),
        high_water_mark: 0,
        extra_user_events: false,
    });
}

// git tag: Tz4/4.8/v4.8.2
#[test]
fn streaming_v14_smoke() {
    common_tests(CommonTestConfig {
        trace_path: TRACE_V14,
        expected_trace_format_version: 14,
        expected_platform_cfg_version_minor: 2,
        initial_event_count: 6,
        latest_timestamp: Timestamp::zero(),
        high_water_mark: 0,
        extra_user_events: true,
    });
}

#[test]
fn streaming_v14_garbage_with_trace_restart() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TRACE_V14);
    let trace_data = std::fs::read(path).unwrap();
    let garbage = vec![0x11, 0x22, 0x33];

    let mut data = Vec::new();
    data.extend_from_slice(&garbage);
    data.extend_from_slice(&trace_data);

    let cfg0 = CommonTestConfig {
        trace_path: TRACE_V14,
        expected_trace_format_version: 14,
        expected_platform_cfg_version_minor: 2,
        initial_event_count: 6,
        latest_timestamp: Timestamp::zero(),
        high_water_mark: 0,
        extra_user_events: true,
    };
    let mut reader = data.as_slice();

    let mut rd0 = RecorderData::find(&mut reader).unwrap();
    check_recorder_data(&rd0, &cfg0);

    for _ in 0..66 {
        let _ = rd0.read_event(&mut reader).unwrap().unwrap();
    }
    let next_psf_word = match rd0.read_event(&mut reader) {
        Err(Error::TraceRestarted(endianness)) => endianness,
        res => panic!("Expected TraceRestarted error. {res:?}"),
    };

    let cfg1 = CommonTestConfig {
        trace_path: TRACE_V14,
        expected_trace_format_version: 14,
        expected_platform_cfg_version_minor: 2,
        initial_event_count: 6,
        latest_timestamp: Timestamp::from(Ticks::new(65)),
        high_water_mark: 4,
        extra_user_events: true,
    };
    let rd1 = RecorderData::read_with_endianness(next_psf_word, &mut reader).unwrap();
    check_recorder_data(&rd1, &cfg1);

    {
        use EventType::*;
        let mut trd = TestRecorderData {
            rd: rd1,
            f: reader,
            event_cnt: 91,
            timestamp_ticks: 66,
        };
        trd.check_event(TraceStart);
    }
}

struct CommonTestConfig {
    trace_path: &'static str,
    expected_trace_format_version: u16,
    expected_platform_cfg_version_minor: u8,
    initial_event_count: u16,
    latest_timestamp: Timestamp,
    high_water_mark: u32,
    extra_user_events: bool,
}

fn check_recorder_data(rd: &RecorderData, cfg: &CommonTestConfig) {
    assert_eq!(rd.protocol, Protocol::Streaming);

    let kernel_version: [u8; 2] = rd.header.kernel_version.into();
    assert_eq!(kernel_version, [0xA1, 0x1A]);
    assert_eq!(
        rd.header,
        HeaderInfo {
            endianness: Endianness::Little,
            format_version: cfg.expected_trace_format_version,
            kernel_version: rd.header.kernel_version,
            kernel_port: KernelPortIdentity::FreeRtos,
            options: 4,
            irq_priority_order: 0,
            num_cores: 1,
            isr_tail_chaining_threshold: 0,
            platform_cfg: "FreeRTOS".to_owned(),
            platform_cfg_version: PlatformCfgVersion {
                major: 1,
                minor: cfg.expected_platform_cfg_version_minor,
                patch: 0,
            },
        }
    );

    assert_eq!(u32::from(rd.timestamp_info.timer_frequency), 1000000_u32);
    assert_eq!(u32::from(rd.timestamp_info.os_tick_rate_hz), 1000_u32);
    assert_eq!(
        rd.timestamp_info,
        TimestampInfo {
            timer_type: TimerCounter::FreeRunning32Incr,
            timer_frequency: rd.timestamp_info.timer_frequency,
            timer_period: 0,
            timer_wraparounds: 0,
            os_tick_rate_hz: rd.timestamp_info.os_tick_rate_hz,
            latest_timestamp: cfg.latest_timestamp,
            os_tick_count: 0,
        },
    );

    assert_eq!(
        rd.entry_table
            .symbol(ObjectHandle::NO_TASK)
            .unwrap()
            .as_ref(),
        STARTUP_TASK_NAME
    );
    assert_eq!(
        rd.entry_table.class(ObjectHandle::NO_TASK).unwrap(),
        ObjectClass::Task,
    );

    assert_eq!(
        rd.system_heap(),
        &Heap {
            current: 0,
            high_water_mark: cfg.high_water_mark,
            max: 32768,
        }
    );
}

fn common_tests(cfg: CommonTestConfig) {
    let mut f = open_trace_file(cfg.trace_path);
    let rd = RecorderData::find(&mut f).unwrap();

    check_recorder_data(&rd, &cfg);

    {
        use EventType::*;
        let mut trd = TestRecorderData {
            rd,
            f,
            event_cnt: cfg.initial_event_count,
            timestamp_ticks: 0,
        };
        trd.check_event(TraceStart);
        trd.check_event(ObjectName);
        trd.check_event(ObjectName);
        trd.check_event(TaskCreate);
        trd.check_event(ObjectName);
        trd.check_event(TaskCreate);
        trd.check_event(DefineIsr);
        trd.check_event(QueueCreate);
        trd.check_event(ObjectName);
        trd.check_event(SemaphoreBinaryCreate);
        trd.check_event(ObjectName);
        trd.check_event(SemaphoreCountingCreate);
        trd.check_event(ObjectName);
        trd.check_event(TaskReady);
        trd.check_event(TaskActivate);
        trd.check_event(QueueSend);
        trd.check_event(QueueSendBlock);
        trd.check_event(QueueSendFront);
        trd.check_event(QueueSendFrontBlock);
        trd.check_event(SemaphoreGive);
        trd.check_event(SemaphoreGive);
        trd.check_event(SemaphoreGiveBlock);
        trd.check_event(SemaphoreGiveBlock);
        trd.check_event(MemoryAlloc);
        trd.check_event(MemoryFree);
        trd.check_event(TaskSwitchIsrBegin);
        trd.check_event(QueueSendFromIsr);
        trd.check_event(QueueSendFrontFromIsr);
        trd.check_event(SemaphoreGiveFromIsr);
        trd.check_event(SemaphoreGiveFromIsr);
        trd.check_event(TaskActivate);
        trd.check_event(TaskReady);
        trd.check_event(TaskActivate);
        trd.check_event(QueueReceive);
        trd.check_event(QueueReceiveBlock);
        trd.check_event(QueueReceiveFromIsr);
        trd.check_event(QueuePeek);
        trd.check_event(QueuePeekBlock);
        trd.check_event(SemaphoreTake);
        trd.check_event(SemaphoreTake);
        trd.check_event(SemaphoreTakeBlock);
        trd.check_event(SemaphoreTakeBlock);
        trd.check_event(SemaphorePeek);
        trd.check_event(SemaphorePeek);
        trd.check_event(SemaphorePeekBlock);
        trd.check_event(SemaphorePeekBlock);
        trd.check_event(SemaphoreTakeFromIsr);
        trd.check_event(SemaphoreTakeFromIsr);
        trd.check_event(UserEvent(3.into()));
        if cfg.extra_user_events {
            trd.check_event(UserEvent(10.into()));
            trd.check_event(ObjectName);
            trd.check_event(ObjectName);
            trd.check_event(UserEvent(8.into()));
            trd.check_event(ObjectName);
            trd.check_event(UserEvent(9.into()));
            trd.check_event(ObjectName);
            trd.check_event(UserEvent(10.into()));
            trd.check_event(ObjectName);
            trd.check_event(UserEvent(11.into()));
            trd.check_event(ObjectName);
            trd.check_event(UserEvent(12.into()));
        }
        trd.check_event(TaskDelay);
        trd.check_event(QueueReceiveBlock);
        trd.check_event(UnusedStack);
    }
}
