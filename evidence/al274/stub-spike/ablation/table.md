| proof | variant | wall-clock s | peak RSS MiB | verbose/event lines | max unwind | product/testkit events | product loop iter | verdict | memchr | alloc_rt_error |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| wav_reads_never_panics_unsigned | baseline-control | 662 | 5818 | 8175/8165 | 17 | 0 | 0 | timeout | 1124 | 254 |
| spike_wav_unsigned_fmt | fmt-only | 662 | 7883 | 1948/1937 | 16 | 32 | 16 | timeout | 0 | 32 |
| spike_wav_unsigned_alloc | alloc-only | 663 | 6198 | 7974/7963 | 17 | 0 | 0 | timeout | 1124 | 0 |
| spike_wav_unsigned_both | combined | 662 | 7869 | 1917/1905 | 16 | 32 | 16 | timeout | 0 | 0 |
| read_wav_range_never_panics_signed | baseline-control | 662 | 8471 | 17705/17695 | 41 | 0 | 0 | timeout | 5279 | 20 |
| spike_wav_range_fmt | fmt-only | 663 | 6744 | 2419/2408 | 19 | 38 | 19 | timeout | 0 | 40 |
| spike_wav_range_alloc | alloc-only | 663 | 6321 | 13183/13172 | 41 | 0 | 0 | timeout | 3958 | 0 |
| spike_wav_range_both | combined | 663 | 6701 | 2367/2355 | 19 | 38 | 19 | timeout | 0 | 0 |
| mp4_writer_output_decodes | baseline-control | 662 | 4356 | 2557/2547 | 48 | 118 | 0 | timeout | 0 | 124 |
| spike_mp4_writer_fmt | fmt-only | 663 | 4298 | 2506/2495 | 48 | 112 | 0 | timeout | 0 | 124 |
| spike_mp4_writer_alloc | alloc-only | 663 | 4298 | 2389/2378 | 48 | 113 | 0 | timeout | 0 | 0 |
| spike_mp4_writer_both | combined | 663 | 4303 | 2395/2383 | 48 | 113 | 0 | timeout | 0 | 0 |
