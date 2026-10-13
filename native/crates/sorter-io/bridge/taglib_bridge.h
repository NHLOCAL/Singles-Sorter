#ifndef SORTER_METADATA_BRIDGE_V1_H
#define SORTER_METADATA_BRIDGE_V1_H
#include <stdint.h>
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Owned, read-only ABI v1. All pointers must be valid for stated lengths.
 * Input/context lives until sm_close. Results belong to this allocator; free once
 * with sm_result_free. Null close/free is valid. Handles are not thread-safe.
 * Never expose save/set/tag-write operations. Every export catches exceptions. */
typedef int32_t (*sm_read_at)(void*, uint64_t, uint8_t*, size_t, size_t*);
typedef struct { uint32_t version; uint32_t struct_size; void *context; sm_read_at read_at; uint64_t length; } SmInput;
typedef struct SmHandle SmHandle;
typedef struct { uint8_t *data; size_t len; } SmResult;
/* 0 success, 1 invalid ABI/argument, 2 parser error, 3 unsupported class,
 * 4 input/budget error, 5 caught native exception. */
int32_t sm_open(const SmInput*, SmHandle**);
int32_t sm_get(SmHandle*, uint32_t field, SmResult*);
void sm_result_free(SmResult*);
void sm_close(SmHandle*);
#ifdef __cplusplus
}
#endif
#endif
