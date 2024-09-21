#!/usr/bin/env bpftrace

BEGIN
{
    printf("Tracing memory allocations for PID %d. Hit Ctrl-C to end.\n", $1);
    @total_alloc = (uint64)0;
}

tracepoint:kmem:kmalloc
/pid == $1/
{
    @allocs[comm] = count();
    @sizes[comm] = hist((uint64)args->bytes_alloc);
    @total_alloc += (uint64)args->bytes_alloc;
}

tracepoint:kmem:kmem_cache_alloc
/pid == $1/
{
    @cache_allocs[comm] = count();
    @cache_sizes[comm] = hist((uint64)args->bytes_alloc);
    @total_alloc += (uint64)args->bytes_alloc;
}

tracepoint:kmem:kfree
/pid == $1/
{
    @frees[comm] = count();
}

tracepoint:kmem:kmem_cache_free
/pid == $1/
{
    @cache_frees[comm] = count();
}

END
{
    printf("\nDirect allocation counts by process:\n");
    print(@allocs);

    printf("\nDirect allocation size distributions:\n");
    print(@sizes);

    printf("\nCache allocation counts by process:\n");
    print(@cache_allocs);

    printf("\nCache allocation size distributions:\n");
    print(@cache_sizes);

    printf("\nDirect free counts by process:\n");
    print(@frees);

    printf("\nCache free counts by process:\n");
    print(@cache_frees);

    printf("\nTotal memory allocated: %lu bytes\n", @total_alloc);
    printf("Note: Total memory freed cannot be accurately tracked with this method.\n");
    printf("Net memory change cannot be calculated accurately.\n");
}