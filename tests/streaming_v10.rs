#![deny(warnings, clippy::all)]

use std::fs::File;
use std::path::Path;
use trace_recorder_parser::{streaming::event::*, streaming::*, time::*, types::*};

const TRACE: &str = "test_resources/fixtures/streaming/v10/trace.psf";

fn open_trace_file(trace_path: &str) -> File {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(trace_path);
    File::open(&path).unwrap()
}

struct TestRecorderData {
    rd: RecorderData,
    f: File,
    event_cnt: u16,
    timestamp_ticks: u64,
}

impl TestRecorderData {
    pub fn check_event(&mut self, typ: EventType) {
        let (ec, ev) = self.rd.read_event(&mut self.f).unwrap().unwrap();
        assert_eq!(ec.event_type(), typ);
        assert_eq!(u16::from(ev.event_count()), self.event_cnt);
        assert_eq!(ev.timestamp().ticks(), self.timestamp_ticks);

        self.event_cnt += 1;
        self.timestamp_ticks += 1;
    }
}

#[test]
fn streaming_v10_smoke() {
    let mut f = open_trace_file(TRACE);
    let rd = RecorderData::read(&mut f).unwrap();

    assert_eq!(rd.protocol, Protocol::Streaming);

    let kernel_version: [u8; 2] = rd.header.kernel_version.into();
    assert_eq!(kernel_version, [0xA1, 0x1A]);
    assert_eq!(
        rd.header,
        HeaderInfo {
            endianness: Endianness::Little,
            format_version: 10,
            kernel_version: rd.header.kernel_version,
            kernel_port: KernelPortIdentity::FreeRtos,
            options: 4,
            irq_priority_order: 0,
            num_cores: 1,
            isr_tail_chaining_threshold: 0,
            platform_cfg: "FreeRTOS".to_owned(),
            platform_cfg_version: PlatformCfgVersion {
                major: 1,
                minor: 0,
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
            latest_timestamp: Timestamp::zero(),
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
            high_water_mark: 0,
            max: 32768,
        }
    );

    {
        use EventType::*;
        let mut trd = TestRecorderData {
            rd,
            f,
            event_cnt: 1,
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
        trd.check_event(Unknown(0x7A.into()));
        trd.check_event(QueueReceiveBlock);
        trd.check_event(UnusedStack);
    }
}
