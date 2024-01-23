#ifndef FREERTOS_CONFIG_H
#define FREERTOS_CONFIG_H

#define configASSERT_DEFINED                             1
void vAssertCalled(const char* file, int line);
#define configASSERT( x )    if( ( x ) == 0 ) vAssertCalled(__FILE__, __LINE__)
#define configQUEUE_REGISTRY_SIZE                        20

#define configUSE_PREEMPTION                             1
#define configUSE_TIME_SLICING                           1
#define configUSE_PORT_OPTIMISED_TASK_SELECTION          0

#define configUSE_IDLE_HOOK                              0
#define configUSE_TICK_HOOK                              0
#define configUSE_DAEMON_TASK_STARTUP_HOOK               0
#define configCPU_CLOCK_HZ                               ( ( unsigned long ) 1000000 )
#define configTICK_RATE_HZ                               ( ( TickType_t ) 1000 )
#define configMINIMAL_STACK_SIZE                         ( ( unsigned short ) 4096 )
#define configTOTAL_HEAP_SIZE                            ( ( size_t ) ( 32 * 1024 ) )
#define configMAX_TASK_NAME_LEN                          ( 12 )
#define configUSE_16_BIT_TICKS                           0
#define configIDLE_SHOULD_YIELD                          1
#define configUSE_CO_ROUTINES                            0
#define configUSE_APPLICATION_TASK_TAG                   1
#define configCHECKFORSTACK_OVERFLOW                     2
#define configUSE_TRACE_FACILITY                         1

#define configGENERATE_RUN_TIME_STATS                    0
#define configRECORD_STACK_HIGH_ADDRESS                  0

#define configMAX_PRIORITIES                             ( 10 )
#define configMAX_CO_ROUTINE_PRIORITIES                  ( 2 )
#define configTIMER_QUEUE_LENGTH                         20
#define configTIMER_TASK_PRIORITY                        ( configMAX_PRIORITIES - 1 )
#define configUSE_COUNTING_SEMAPHORES                    1
#define configSUPPORT_DYNAMIC_ALLOCATION                 1
#define configSUPPORT_STATIC_ALLOCATION                  0
#define configNUM_TX_DESCRIPTORS                         15

#define configUSE_MALLOC_FAILED_HOOK              1
#define configUSE_MUTEXES                         1
#define configUSE_RECURSIVE_MUTEXES               1
#define configUSE_TIMERS                          1
#define configTIMER_TASK_STACK_DEPTH              ( configMINIMAL_STACK_SIZE * 2 )

#define INCLUDE_vTaskPrioritySet                  1
#define INCLUDE_uxTaskPriorityGet                 1
#define INCLUDE_vTaskDelete                       0
#define INCLUDE_vTaskCleanUpResources             0
#define INCLUDE_vTaskSuspend                      0
#define INCLUDE_vTaskDelay                        1
#define INCLUDE_xTaskDelayUntil                   1
#define INCLUDE_uxTaskGetStackHighWaterMark       0
#define INCLUDE_uxTaskGetStackHighWaterMark2      0
#define INCLUDE_xTaskGetSchedulerState            0
#define INCLUDE_xTaskGetIdleTaskHandle            0
#define INCLUDE_xTaskGetHandle                    1
#define INCLUDE_eTaskGetState                     0
#define INCLUDE_xSemaphoreGetMutexHolder          0
#define INCLUDE_xTimerPendFunctionCall            0
#define INCLUDE_xTaskAbortDelay                   0
#define INCLUDE_xTaskGetCurrentTaskHandle         1

#define projCOVERAGE_TEST                         0

#define configKERNEL_INTERRUPT_PRIORITY           252

/* !!!! configMAX_SYSCALL_INTERRUPT_PRIORITY must not be set to zero !!!!
 * See http://www.FreeRTOS.org/RTOS-Cortex-M3-M4.html. */
#define configMAX_SYSCALL_INTERRUPT_PRIORITY    5 /* equivalent to 0xa0, or priority 5. */
#define configMAC_INTERRUPT_PRIORITY            2

#include "trcRecorder.h"

#endif /* FREERTOS_CONFIG_H */
