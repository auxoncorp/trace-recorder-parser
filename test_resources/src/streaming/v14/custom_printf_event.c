#include <stdio.h>
#include <stdarg.h>
#include <string.h>
#include <assert.h>

#include "FreeRTOS.h"

#undef configASSERT
#define configASSERT(x) assert(x)

#define CUSTOM_PRINTF_EVENT_ID (0x0FA0)
#define MAX_ARGS (16)
#define MAX_FMT_STR_LEN (128)

#if (TRC_CFG_CORE_COUNT > 1)
#define TRC_EVENT_SET_EVENT_COUNT(c)  ((uint16_t)(((TRC_CFG_GET_CURRENT_CORE() & 0xF) << 12) | ((uint16_t)(c) & 0xFFF)))
#else
#define TRC_EVENT_SET_EVENT_COUNT(c) ((uint16_t)(c))
#endif

#define TRC_EVENT_SET_PARAM_COUNT(id, n) ((uint16_t)(((uint16_t)(id)) | ((((uint16_t)(n)) & 0xF) << 12)))

typedef struct
{
    uint16_t event_id;
    uint16_t event_count;
    uint32_t timestamp;
    uint32_t channel;
    uint16_t args_len;
    uint16_t fmt_len;
    // Followed by
    // TraceUnsignedBaseType_t params[]
    // uint8_t fmt_str[]
} custom_printf_event_header_s;

extern TraceEventDataTable_t *pxTraceEventDataTable;
extern TraceStringHandle_t g_log_ch;

traceResult custom_vprintf(const char* fmt, va_list* args)
{
    uint32_t i;
    traceResult ret;
    TraceUnsignedBaseType_t arg;
    TraceStringHandle_t str_handle;
    uint32_t event_size;
    int32_t bytes_committed;
    void* event_data;
    custom_printf_event_header_s header;
    TRACE_ALLOC_CRITICAL_SECTION();

    TraceUnsignedBaseType_t args_list[MAX_ARGS];
    TraceUnsignedBaseType_t interned_string_args[MAX_ARGS] = {0};

    configASSERT(CUSTOM_PRINTF_EVENT_ID > TRC_EVENT_LAST_ID);
    configASSERT(xTraceIsComponentInitialized(TRC_RECORDER_COMPONENT_PRINT) != 0);
    configASSERT(g_log_ch != NULL);
    configASSERT(fmt != NULL);
    configASSERT(args != NULL);

    // Count the args in the format string
    header.args_len = 0;
    for(i = 0; (fmt[i] != (char) 0) && (i < MAX_FMT_STR_LEN); i++)
    {
        if(fmt[i] == '%')
        {
            if(fmt[i + 1] == (char) 0)
            {
                // End of fmt string
                continue;
            }

            if(fmt[i + 1] != '%')
            {
                // Flag this arg as a string to be interned
                if(fmt[i + 1] == 's')
                {
                    if(header.args_len < MAX_ARGS)
                    {
                        interned_string_args[header.args_len] = 1;
                    }
                }

                header.args_len += 1;
            }

            // Move past format specifier
            i += 1;
        }
    }

    header.fmt_len = i;

    // Truncate args
    if(header.args_len > MAX_ARGS)
    {
        header.args_len = MAX_ARGS;
    }

    // Walk the arg list, interning any string args along the way
    for(i = 0; i < header.args_len; i += 1)
    {
        arg = va_arg(*args, TraceUnsignedBaseType_t);

        // Attempt to intern if a string arg
        if(interned_string_args[i] != 0)
        {
            if(xTraceStringRegister((const char*) arg, &str_handle) == TRC_SUCCESS)
            {
                configASSERT(str_handle != 0);
                interned_string_args[i] = (TraceUnsignedBaseType_t) str_handle;
                args_list[i] = interned_string_args[i];
            }
            else
            {
                // Clear it so we don't try and free it from the entry table
                interned_string_args[i] = 0;

                // Invalid entry pointer
                args_list[i] = 0;
            }
        }
        else
        {
            args_list[i] = arg;
        }
    }

    TRACE_ENTER_CRITICAL_SECTION();

    header.event_id = TRC_EVENT_SET_PARAM_COUNT(CUSTOM_PRINTF_EVENT_ID, 0);
    pxTraceEventDataTable->coreEventData[TRC_CFG_GET_CURRENT_CORE()].eventCounter++;
    header.event_count = TRC_EVENT_SET_EVENT_COUNT(pxTraceEventDataTable->coreEventData[TRC_CFG_GET_CURRENT_CORE()].eventCounter);
    xTraceTimestampGet(&header.timestamp);
    header.channel = (uint32_t) g_log_ch;

    event_size = sizeof(header)
        + (sizeof(TraceUnsignedBaseType_t) * header.args_len)
        + header.fmt_len;

    ret = xTraceStreamPortAllocate(event_size, &event_data);
    if(ret == TRC_SUCCESS)
    {
        memcpy(event_data, &header, sizeof(header));
        memcpy(
                event_data + sizeof(header),
                args_list,
                sizeof(TraceUnsignedBaseType_t) * header.args_len);
        memcpy(
                event_data + sizeof(header) + (sizeof(TraceUnsignedBaseType_t) * header.args_len),
                fmt,
                header.fmt_len);

        (void) xTraceStreamPortCommit(event_data, event_size, &bytes_committed);
    }

    TRACE_EXIT_CRITICAL_SECTION();

    // Free up the interned strings from the entry table
    for(i = 0; i < header.args_len; i += 1)
    {
        if(interned_string_args[i] != 0)
        {
            xTraceEntryDelete((TraceEntryHandle_t) interned_string_args[i]);
        }
    }

    return ret;
}

void custom_printf(const char* fmt, ...)
{
    traceResult tr;
    va_list args;

    if(xTraceIsRecorderEnabled() != 0)
    {
        va_start(args, fmt);
        tr = custom_vprintf(fmt, &args);
        va_end(args);
        configASSERT(tr == TRC_SUCCESS);
    }
}

