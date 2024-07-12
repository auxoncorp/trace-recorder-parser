#ifdef NDEBUG
#undef NDEBUG
#endif

#include "FreeRTOS.h"
#include "task.h"
#include "queue.h"
#include "semphr.h"

#include <stdlib.h>
#include <assert.h>
#include <errno.h>

#include "trcTypes.h"
#include "trc_mock.h"

#if (TRC_CFG_USE_TRACE_ASSERT != 1)
#error "Set TRC_CFG_USE_TRACE_ASSERT=1"
#endif

static TraceBaseType_t g_timer_ticks = 0;
static TraceStreamPortFile_t* g_trace_file = NULL;
static TraceUnsignedBaseType_t g_heap = 0xFF00;

static int g_trace_append_mode = 0;

static void* not_traced_heap_ptr(void)
{
    TraceUnsignedBaseType_t ptr = g_heap;
    g_heap += 1;
    return (void*) ptr;
}

void trc_port_init(void)
{
}

uint32_t trc_timer_read(void)
{
    const TraceBaseType_t ticks = g_timer_ticks;
    g_timer_ticks += 1;
    return ticks;
}

TraceBaseType_t trc_enter_critical(void)
{
    return 0;
}

void trc_exit_critical(TraceBaseType_t cr)
{
    (void) cr;
}

UBaseType_t uxTaskGetStackHighWaterMark(TaskHandle_t task)
{
    (void) task;
    assert(configMINIMAL_STACK_SIZE >= 100);
    return 50;
}

void vTaskDelay(const TickType_t xTicksToDelay)
{
    if(xTicksToDelay > 0)
    {
        traceTASK_DELAY();
    }
}

BaseType_t xTaskCreate(
        TaskFunction_t task_code,
        const char * const name,
        const configSTACK_DEPTH_TYPE stack_depth,
        void * const parameters,
        UBaseType_t priority,
        TaskHandle_t * const task)
{
    (void) task_code;
    (void) stack_depth;
    (void) parameters;
    (void) priority;

    assert(task != NULL);
    *task = not_traced_heap_ptr();
    assert(*task != NULL);
    printf("Creating task name='%s', ptr=%p\n", name, *task);
    xTraceTaskRegisterWithoutHandle(*task, name, priority);
    return pdPASS;
}

QueueHandle_t xQueueCreateCountingSemaphore(
        const UBaseType_t uxMaxCount,
        const UBaseType_t uxInitialCount)
{
    assert(uxMaxCount != 0);
    assert(uxInitialCount <= uxMaxCount);
    return xQueueGenericCreate(uxMaxCount, 0, queueQUEUE_TYPE_COUNTING_SEMAPHORE);
}

QueueHandle_t xQueueGenericCreate(
        const UBaseType_t uxQueueLength,
        const UBaseType_t uxItemSize,
        const uint8_t ucQueueType)
{
    void* q = not_traced_heap_ptr();
    assert(q != NULL);
    if(ucQueueType == queueQUEUE_TYPE_BASE)
    {
        printf("Creating queue length=%lu, item_size=%lu, type=%u, ptr=%p\n", uxQueueLength, uxItemSize, (unsigned) ucQueueType, q);
        xTraceObjectRegisterWithoutHandle(PSF_EVENT_QUEUE_CREATE, q, "", (uint32_t)uxQueueLength);
    }
    else if(ucQueueType == queueQUEUE_TYPE_COUNTING_SEMAPHORE)
    {
        printf("Creating counting semaphore length=%lu, item_size=%lu, type=%u, ptr=%p\n", uxQueueLength, uxItemSize, (unsigned) ucQueueType, q);
        xTraceObjectRegisterWithoutHandle(PSF_EVENT_SEMAPHORE_COUNTING_CREATE, q, "", (uint32_t)uxQueueLength);
    }
    else if(ucQueueType == queueQUEUE_TYPE_BINARY_SEMAPHORE)
    {
        printf("Creating binary semaphore length=%lu, item_size=%lu, type=%u, ptr=%p\n", uxQueueLength, uxItemSize, (unsigned) ucQueueType, q);
        xTraceObjectRegisterWithoutHandle(PSF_EVENT_SEMAPHORE_BINARY_CREATE, q, "", 0);
    }
    else
    {
        assert(0);
    }
    return q;
}

BaseType_t xTaskGetSchedulerState(void)
{
    return taskSCHEDULER_NOT_STARTED;
}

traceResult xTraceStreamPortInitialize(TraceStreamPortBuffer_t* pxBuffer)
{
    TRC_ASSERT_EQUAL_SIZE(TraceStreamPortBuffer_t, TraceStreamPortFile_t);

    TRC_ASSERT(pxBuffer != 0);

    g_trace_file = (TraceStreamPortFile_t*)pxBuffer;
    g_trace_file->pxFile = 0;

#if (TRC_USE_INTERNAL_BUFFER == 1)
    return xTraceInternalEventBufferInitialize(g_trace_file->buffer, sizeof(g_trace_file->buffer));
#else
    return TRC_SUCCESS;
#endif
}

traceResult xTraceStreamPortOnTraceBegin(void)
{
    if (g_trace_file == 0)
    {
        return TRC_FAIL;
    }

    if (g_trace_file->pxFile == 0)
    {
        if (g_trace_append_mode == 0)
        {
            g_trace_file->pxFile = fopen(TRC_CFG_STREAM_PORT_TRACE_FILE, "wb");
        }
        else{
            g_trace_file->pxFile = fopen(TRC_CFG_STREAM_PORT_TRACE_FILE, "ab");
        }

        if(g_trace_file->pxFile == NULL)
        {
            printf("Could not open trace file, error code %d\n", errno);
            return TRC_FAIL;
        }
        else
        {
            printf("Created trace file '%s'\n", TRC_CFG_STREAM_PORT_TRACE_FILE);
        }
    }

    return TRC_SUCCESS;
}

traceResult xTraceStreamPortOnTraceEnd(void)
{
    if (g_trace_file == 0)
    {
        return TRC_FAIL;
    }

    if (g_trace_file->pxFile != 0)
    {
        fclose(g_trace_file->pxFile);
        g_trace_file->pxFile = 0;
        printf("Trace file closed\n");
    }

    return TRC_SUCCESS;
}

traceResult xTraceStreamPortWriteData(void* pvData, uint32_t uiSize, int32_t* piBytesWritten)
{
    const size_t ret = fwrite(pvData, 1, uiSize, g_trace_file->pxFile);
    assert(ret == (size_t) uiSize);
    *piBytesWritten = (int32_t) uiSize;
    return TRC_SUCCESS;
}

int main(int argc, char **argv)
{
    (void) argc;
    (void) argv;

    assert(xTraceEnable(TRC_START) == TRC_SUCCESS);

    TraceStringHandle_t ch;
    assert(xTraceStringRegister("channel-foo", &ch) == TRC_SUCCESS);

    TaskHandle_t task_a;
    assert(xTaskCreate(NULL, "TASK_A", configMINIMAL_STACK_SIZE, NULL, tskIDLE_PRIORITY, &task_a) == pdPASS);

    TaskHandle_t task_b;
    assert(xTaskCreate(NULL, "TASK_B", configMINIMAL_STACK_SIZE, NULL, tskIDLE_PRIORITY, &task_b) == pdPASS);

    TraceISRHandle_t isr;
    assert(xTraceISRRegister("ISR", 2, &isr) == TRC_SUCCESS);

    QueueHandle_t q = xQueueCreate(10, sizeof(uint32_t));
    assert(q != NULL);
    vTraceSetQueueName(q, "msg-queue");

    SemaphoreHandle_t bs = xSemaphoreCreateBinary();
    assert(bs != NULL);
    vTraceSetSemaphoreName(bs, "bin-sem");

    SemaphoreHandle_t cs = xSemaphoreCreateCounting(10, 1);
    assert(cs != NULL);
    vTraceSetSemaphoreName(cs, "count-sem");

    assert(xTraceTaskReady(task_a) == TRC_SUCCESS);
    assert(xTraceTaskSwitch(task_a, tskIDLE_PRIORITY) == TRC_SUCCESS);

    prvTraceStoreEvent_HandleParam(PSF_EVENT_QUEUE_SEND, q, 1);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_QUEUE_SEND_BLOCK, q, 2);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_QUEUE_SEND_FRONT, q, 3);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_QUEUE_SEND_FRONT_BLOCK, q, 4);

    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_GIVE, bs, 1);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_GIVE, cs, 1);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_GIVE_BLOCK, bs, 1);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_GIVE_BLOCK, cs, 2);

    TraceUnsignedBaseType_t memsize = sizeof(uint32_t);
    void* mem = not_traced_heap_ptr();
    traceMALLOC(mem, memsize);
    traceFREE(mem, memsize);

    assert(xTraceISRBegin(isr) == TRC_SUCCESS);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_QUEUE_SEND_FROMISR, q, 5);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_QUEUE_SEND_FRONT_FROMISR, q, 6);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_GIVE_FROMISR, bs, 1);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_GIVE_FROMISR, cs, 3);
    assert(xTraceISREnd(0) == TRC_SUCCESS);

    assert(xTraceTaskReady(task_b) == TRC_SUCCESS);
    assert(xTraceTaskSwitch(task_b, tskIDLE_PRIORITY) == TRC_SUCCESS);

    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_QUEUE_RECEIVE, q, pdMS_TO_TICKS(100), 5);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_QUEUE_RECEIVE_BLOCK, q, pdMS_TO_TICKS(100), 5);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_QUEUE_RECEIVE_FROMISR, q, 4);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_QUEUE_PEEK, q, pdMS_TO_TICKS(100), 4);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_QUEUE_PEEK_BLOCK, q, pdMS_TO_TICKS(100), 4);

    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_TAKE, bs, pdMS_TO_TICKS(100), 0);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_TAKE, cs, pdMS_TO_TICKS(100), 2);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_TAKE_BLOCK, bs, pdMS_TO_TICKS(100), 1);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_TAKE_BLOCK, cs, pdMS_TO_TICKS(100), 1);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_PEEK, bs, pdMS_TO_TICKS(100), 0);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_PEEK, cs, pdMS_TO_TICKS(100), 0);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_PEEK_BLOCK, bs, pdMS_TO_TICKS(100), 0);
    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_SEMAPHORE_PEEK_BLOCK, cs, pdMS_TO_TICKS(100), 0);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_TAKE_FROMISR, bs, 0);
    prvTraceStoreEvent_HandleParam(PSF_EVENT_SEMAPHORE_TAKE_FROMISR, cs, 0);

    assert(xTracePrintF(ch, "int %d, unsigned %u", -2, 32) == TRC_SUCCESS);
    // Exceed the PSF_EVENT_USER_EVENT_FIXED id
    assert(xTracePrintF(ch, "%u %u %u %u %u %u %u %u %u", 1, 2, 3, 4, 5, 6, 7, 8, 9) == TRC_SUCCESS);

    TraceStringHandle_t ch1;
    assert(xTraceStringRegister("ch1", &ch1) == TRC_SUCCESS);

    TraceStringHandle_t fmt0;
    assert(xTraceStringRegister("no args", &fmt0) == TRC_SUCCESS);
    assert(xTracePrintF0(ch1, fmt0) == TRC_SUCCESS);

    TraceStringHandle_t fmt1;
    assert(xTraceStringRegister("1 arg: %u", &fmt1) == TRC_SUCCESS);
    assert(xTracePrintF1(ch1, fmt1, 0) == TRC_SUCCESS);

    TraceStringHandle_t fmt2;
    assert(xTraceStringRegister("2 args: %u %u", &fmt2) == TRC_SUCCESS);
    assert(xTracePrintF2(ch1, fmt2, 1, 2) == TRC_SUCCESS);

    TraceStringHandle_t fmt3;
    assert(xTraceStringRegister("3 args: %u %u %u", &fmt3) == TRC_SUCCESS);
    assert(xTracePrintF3(ch1, fmt3, 1, 2, 3) == TRC_SUCCESS);

    TraceStringHandle_t fmt4;
    assert(xTraceStringRegister("4 args: %u %u %u %u", &fmt4) == TRC_SUCCESS);
    assert(xTracePrintF4(ch1, fmt4, 1, 2, 3, 4) == TRC_SUCCESS);

    vTaskDelay(pdMS_TO_TICKS(25));

    prvTraceStoreEvent_HandleParamParam(PSF_EVENT_QUEUE_RECEIVE_BLOCK, q, pdMS_TO_TICKS(100), 0);

    assert(xTraceStackMonitorReport() == TRC_SUCCESS);

    assert(xTraceDiagnosticsCheckStatus() == TRC_SUCCESS);

    assert(xTraceDisable() == TRC_SUCCESS);

    /* restart */
    g_trace_append_mode = 1;
    assert(xTraceEnable(TRC_START) == TRC_SUCCESS);
    assert(xTracePrintF(ch, "int %d, unsigned %u", -2, 32) == TRC_SUCCESS);
    assert(xTraceDisable() == TRC_SUCCESS);

    return EXIT_SUCCESS;
}
