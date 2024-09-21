#!/usr/bin/env bpftrace

#include <linux/sched.h>

BEGIN
{
    printf("Sampling memory usage for PID %d every second. Hit Ctrl-C to end.\n", $1);
    @target_pid = $1;
}

tracepoint:sched:sched_process_exit
/@target_pid == args->pid/
{
    exit();
}

interval:s:1
{
    $task = (struct task_struct *)curtask;
    if ($task->pid == @target_pid)
    {
        $mm = $task->mm;
        if ($mm != 0)
        {
            @vss = $mm->total_vm * 4096;
            @rss = $mm->_resident_pages_index * 4096;
            @data = ($mm->end_data - $mm->start_data);
            @stack = ($task->thread.sp0 - $task->thread.sp);

            printf("%llu,%lu,%lu,%lu,%lu\n",
                   nsecs,
                   @vss,
                   @rss,
                   @data,
                   @stack
            );
        }
    }
}

END
{
    clear(@target_pid);
    clear(@vss);
    clear(@rss);
    clear(@data);
    clear(@stack);
}