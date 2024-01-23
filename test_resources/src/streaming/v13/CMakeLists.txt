cmake_minimum_required(VERSION 3.5)

project(trc-streaming-v13 LANGUAGES C)

if(NOT CMAKE_C_STANDARD)
  set(CMAKE_C_STANDARD 99)
endif()

set(
    FREERTOS_TAG
    "V10.5.0"
    CACHE
    STRING
    "Sets the FreeRTOS git tag to use")

if(NOT EXISTS "${CMAKE_SOURCE_DIR}/FreeRTOS")
    message(STATUS "Cloning FreeRTOS ${FREERTOS_TAG}")
    execute_process(
        COMMAND git clone --depth 1 --recurse-submodules -j 4 --branch "${FREERTOS_TAG}" https://github.com/FreeRTOS/FreeRTOS-Kernel.git "${CMAKE_SOURCE_DIR}/FreeRTOS"
        WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
        ERROR_QUIET
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE FREERTOS_CLONE_RESULT
        OUTPUT_VARIABLE FREERTOS_CLONE_ERROR
        ERROR_VARIABLE FREERTOS_CLONE_ERROR)
    if(FREERTOS_CLONE_RESULT AND NOT FREERTOS_CLONE_RESULT EQUAL 0)
        message(FATAL_ERROR "Cloning FreeRTOS ${FREERTOS_TAG} failed:\n${FREERTOS_CLONE_ERROR}")
    endif()
endif()

set(
    TRACE_RECORDER_TAG
    "Tz4/4.8/v4.8.0.hotfix1"
    CACHE
    STRING
    "Sets the TraceRecorderSource git tag to use")

if(NOT EXISTS "${CMAKE_SOURCE_DIR}/TraceRecorderSource")
    message(STATUS "Cloning TraceRecorderSource ${TRACE_RECORDER_TAG}")
    execute_process(
        COMMAND git clone --depth 1 --recurse-submodules -j 4 --branch "${TRACE_RECORDER_TAG}" https://github.com/percepio/TraceRecorderSource.git "${CMAKE_SOURCE_DIR}/TraceRecorderSource"
        WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
        ERROR_QUIET
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE TRC_CLONE_RESULT
        OUTPUT_VARIABLE TRC_CLONE_ERROR
        ERROR_VARIABLE TRC_CLONE_ERROR)
    if(TRC_CLONE_RESULT AND NOT TRC_CLONE_RESULT EQUAL 0)
        message(FATAL_ERROR "Cloning TraceRecorderSource ${TRACE_RECORDER_TAG} failed:\n${TRC_CLONE_ERROR}")
    endif()
endif()

set(
    KERNEL_INCLUDES
    ${CMAKE_SOURCE_DIR}/FreeRTOS/include
    ${CMAKE_SOURCE_DIR}/FreeRTOS/portable/ThirdParty/GCC/Posix
    ${CMAKE_SOURCE_DIR}/FreeRTOS/portable/ThirdParty/GCC/Posix/utils)

set(
    TRACING_SOURCES
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/kernelports/FreeRTOS/trcKernelPort.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcAssert.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcCounter.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcDiagnostics.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcEntryTable.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcError.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcEventBuffer.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcEvent.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcExtension.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcHardwarePort.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcHeap.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcInternalEventBuffer.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcInterval.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcISR.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcMultiCoreEventBuffer.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcObject.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcPrint.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcStackMonitor.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcStateMachine.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcStaticBuffer.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcString.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcTask.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcTimestamp.c
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/trcStreamingRecorder.c)
set(
    TRACING_INCLUDES
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/kernelports/FreeRTOS/include
    ${CMAKE_SOURCE_DIR}/TraceRecorderSource/include)

add_executable(
    ${PROJECT_NAME}
    main.c
    ${TRACING_SOURCES})

target_include_directories(
    ${PROJECT_NAME}
    PRIVATE
    ${CMAKE_SOURCE_DIR}
    ${KERNEL_INCLUDES}
    ${TRACING_INCLUDES})

target_compile_definitions(
    ${PROJECT_NAME}
    PRIVATE
    TRC_CFG_TEST_MODE
    TRC_CFG_STREAM_PORT_TRACE_FILE="${CMAKE_SOURCE_DIR}/../../../fixtures/streaming/v13/trace.psf")

target_compile_options(
    ${PROJECT_NAME}
    PRIVATE
    -m32
    -Wall -Wextra -Werror
    -Wno-incompatible-pointer-types
    -Wno-int-to-pointer-cast
    -Wno-pointer-to-int-cast)

target_link_options(
    ${PROJECT_NAME}
    PRIVATE
    -m32)

add_custom_target(
    run
    DEPENDS ${PROJECT_NAME})

add_custom_command(
    TARGET run
    POST_BUILD
    MAIN_DEPENDENCY ${PROJECT_NAME}
    COMMAND ./${PROJECT_NAME}
    COMMENT "Running the v13 test application")
