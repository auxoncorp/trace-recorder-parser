#ifndef TRC_MOCK_H
#define TRC_MOCK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define TRC_BASE_TYPE int32_t
#define TRC_UNSIGNED_BASE_TYPE uint32_t

#define TRC_HWTC_TYPE TRC_FREE_RUNNING_32BIT_INCR
#define TRC_HWTC_COUNT (trc_timer_read())
#define TRC_HWTC_PERIOD 0
#define TRC_HWTC_DIVISOR 1
#define TRC_HWTC_FREQ_HZ (TRACE_CPU_CLOCK_HZ)
#define TRC_IRQ_PRIORITY_ORDER 0

#define TRC_PORT_SPECIFIC_INIT() trc_port_init()

#define TRACE_ALLOC_CRITICAL_SECTION() TraceBaseType_t TRACE_ALLOC_CRITICAL_SECTION_NAME;
#define TRACE_ENTER_CRITICAL_SECTION() { TRACE_ALLOC_CRITICAL_SECTION_NAME = trc_enter_critical(); }
#define TRACE_EXIT_CRITICAL_SECTION() { trc_exit_critical(TRACE_ALLOC_CRITICAL_SECTION_NAME); }

void trc_port_init(void);
uint32_t trc_timer_read(void);
TRC_BASE_TYPE trc_enter_critical(void);
void trc_exit_critical(TRC_BASE_TYPE cr);

#ifdef __cplusplus
}
#endif

#endif /* TRC_MOCK_H */
