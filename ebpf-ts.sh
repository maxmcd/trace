#!/usr/bin/env bpftrace

BEGIN
{
    printf("Tracing memory allocations for PID %d. Hit Ctrl-C to end.\n", $1);
    @memory_usage = (int64)0;
}

uprobe:/lib/x86_64-linux-gnu/libc.so.6:malloc
/pid == $1/
{
    @malloc_start[tid] = nsecs;
}

uretprobe:/lib/x86_64-linux-gnu/libc.so.6:malloc
/pid == $1 && @malloc_start[tid]/
{
    $size = (int64)retval;
    $duration = nsecs - @malloc_start[tid]; // Duration in nanoseconds
    @memory_usage += $size;
    @allocs[tid] = count();
    @alloc_sizes = hist($size);
    printf("%llu,%d,%d,%d,%llu\n", nsecs, 1, tid, $size, $duration);
    @alloc_map[(uint64)retval] = $size;
    delete(@malloc_start[tid]);
}

uprobe:/lib/x86_64-linux-gnu/libc.so.6:free
/pid == $1/
{
    $addr = (uint64)arg0;
    if ($addr != 0) {
        $size = @alloc_map[$addr];
        if ($size != 0) {
            @memory_usage -= $size;
            @frees[tid] = count();
            @free_sizes = hist($size);
            printf("%llu,%d,%d,%d\n", nsecs, 0, tid, $size);
            delete(@alloc_map[$addr]);
        }
    }
}

END
{
    printf("\nAllocation counts by thread:\n");
    print(@allocs);

    printf("\nAllocation size distributions:\n");
    print(@alloc_sizes);

    printf("\nFree counts by thread:\n");
    print(@frees);

    printf("\nFree size distributions:\n");
    print(@free_sizes);

    printf("\nFinal memory usage: %ld bytes\n", @memory_usage);

    clear(@malloc_start);
    clear(@alloc_map);
}