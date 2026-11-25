# Top verbose-log functions for selected ablation rows

Counts are lines in the CBMC verbose log containing ` function <name>`; they are event-noise proxies, not solver time percentages.

## wav_reads_never_panics_unsigned

| count | function |
| ---: | --- |
| 2283 | `<usize as kani::rustc_intrinsics::ToISize>::to_isize` |
| 1041 | `kani::mem::cbmc::same_allocation` |
| 836 | `core::slice::memchr::memchr_naive` |
| 381 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::handle_error` |
| 273 | `memcmp` |
| 272 | `core::slice::memchr::memchr_aligned::runtime` |
| 254 | `std::alloc::handle_alloc_error::rt_error` |
| 248 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::<A>::reserve::do_reserve_and_handle::<std::alloc::Global>` |
| 128 | `std::result::Result::<std::alloc::Layout, std::alloc::LayoutError>::map_err::<std::collections::TryReserveError, {closure@decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::layout_array::{closure#0}}>` |
| 127 | `<std::collections::TryReserveErrorKind as std::clone::Clone>::clone` |
| 127 | `std::alloc::handle_alloc_error` |
| 127 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::capacity_overflow` |
| 125 | `<std::result::Result<std::alloc::Layout, std::collections::TryReserveError> as std::ops::Try>::branch` |
| 125 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::finish_grow` |
| 125 | `<std::result::Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> as std::ops::Try>::branch` |

## spike_wav_unsigned_fmt

| count | function |
| ---: | --- |
| 228 | `<usize as kani::rustc_intrinsics::ToISize>::to_isize` |
| 185 | `kani::mem::cbmc::same_allocation` |
| 122 | `core::slice::<impl [u8]>::split_at` |
| 120 | `core::slice::index::slice_index_fail::do_panic::runtime` |
| 120 | `core::slice::index::slice_index_fail::do_panic` |
| 120 | `core::slice::index::slice_index_fail` |
| 64 | `std::option::Option::<&[u8]>::map::<u16, {closure@al_media::wav::read_u16::{closure#0}}>` |
| 64 | `std::option::Option::<u16>::ok_or_else::<al_media::MediaError, {closure@al_media::wav::read_u16::{closure#1}}>` |
| 64 | `<std::result::Result<u16, al_media::MediaError> as std::ops::Try>::branch` |
| 48 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::handle_error` |
| 32 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::with_capacity_zeroed_in` |
| 32 | `std::alloc::handle_alloc_error::rt_error` |
| 32 | `std::option::Option::<&[u8]>::map::<u32, {closure@al_media::wav::read_u32::{closure#0}}>` |
| 32 | `std::option::Option::<u32>::ok_or_else::<al_media::MediaError, {closure@al_media::wav::read_u32::{closure#1}}>` |
| 32 | `<std::result::Result<u32, al_media::MediaError> as std::ops::Try>::branch` |

## read_wav_range_never_panics_signed

| count | function |
| ---: | --- |
| 7832 | `<usize as kani::rustc_intrinsics::ToISize>::to_isize` |
| 3964 | `core::slice::memchr::memchr_naive` |
| 2632 | `kani::mem::cbmc::same_allocation` |
| 1284 | `core::slice::memchr::memchr_aligned::runtime` |
| 1256 | `memcmp` |
| 64 | `core::slice::index::slice_index_fail::do_panic::runtime` |
| 64 | `core::slice::index::slice_index_fail::do_panic` |
| 64 | `core::slice::index::slice_index_fail` |
| 62 | `<std::ops::Range<usize> as std::slice::SliceIndex<[u8]>>::get` |
| 62 | `<std::str::pattern::CharSearcher<'_> as std::str::pattern::Searcher<'_>>::next_match` |
| 32 | `<std::option::Option<&[u8]> as std::ops::Try>::branch` |
| 32 | `<std::ops::Range<usize> as std::slice::SliceIndex<[u8]>>::index` |
| 31 | `core::slice::memchr::memchr` |
| 30 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::handle_error` |
| 20 | `std::alloc::handle_alloc_error::rt_error` |

## spike_wav_range_fmt

| count | function |
| ---: | --- |
| 288 | `<usize as kani::rustc_intrinsics::ToISize>::to_isize` |
| 229 | `kani::mem::cbmc::same_allocation` |
| 154 | `core::slice::<impl [u8]>::split_at` |
| 152 | `core::slice::index::slice_index_fail::do_panic::runtime` |
| 152 | `core::slice::index::slice_index_fail::do_panic` |
| 152 | `core::slice::index::slice_index_fail` |
| 78 | `std::option::Option::<&[u8]>::map::<u16, {closure@al_media::wav::read_u16::{closure#0}}>` |
| 77 | `std::option::Option::<u16>::ok_or_else::<al_media::MediaError, {closure@al_media::wav::read_u16::{closure#1}}>` |
| 77 | `<std::result::Result<u16, al_media::MediaError> as std::ops::Try>::branch` |
| 60 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::handle_error` |
| 40 | `decode_wav_never_panics_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::with_capacity_zeroed_in` |
| 40 | `std::alloc::handle_alloc_error::rt_error` |
| 39 | `core::slice::copy_from_slice_impl::len_mismatch_fail::do_panic::runtime` |
| 39 | `core::slice::copy_from_slice_impl::len_mismatch_fail::do_panic` |
| 39 | `core::slice::copy_from_slice_impl::len_mismatch_fail` |

## mp4_writer_output_decodes

| count | function |
| ---: | --- |
| 766 | `kani::mem::cbmc::same_allocation` |
| 330 | `<usize as kani::rustc_intrinsics::ToISize>::to_isize` |
| 186 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::handle_error` |
| 124 | `std::alloc::handle_alloc_error::rt_error` |
| 118 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::<A>::reserve::do_reserve_and_handle::<std::alloc::Global>` |
| 114 | `al_testkit::mp4::write_mp4` |
| 62 | `std::result::Result::<std::alloc::Layout, std::alloc::LayoutError>::map_err::<std::collections::TryReserveError, {closure@container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::layout_array::{closure#0}}>` |
| 62 | `<std::collections::TryReserveErrorKind as std::clone::Clone>::clone` |
| 62 | `std::alloc::handle_alloc_error` |
| 62 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::capacity_overflow` |
| 59 | `<std::result::Result<std::alloc::Layout, std::collections::TryReserveError> as std::ops::Try>::branch` |
| 59 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::finish_grow` |
| 59 | `<std::result::Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> as std::ops::Try>::branch` |
| 59 | `std::alloc::Global::grow_impl_runtime` |
| 59 | `std::result::Result::<std::ptr::NonNull<[u8]>, std::alloc::AllocError>::map_err::<std::collections::TryReserveError, {closure@container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::finish_grow::{closure#0}}>` |

## spike_mp4_writer_alloc

| count | function |
| ---: | --- |
| 726 | `kani::mem::cbmc::same_allocation` |
| 330 | `<usize as kani::rustc_intrinsics::ToISize>::to_isize` |
| 186 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::handle_error` |
| 118 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::<A>::reserve::do_reserve_and_handle::<std::alloc::Global>` |
| 109 | `al_testkit::mp4::write_mp4` |
| 62 | `std::result::Result::<std::alloc::Layout, std::alloc::LayoutError>::map_err::<std::collections::TryReserveError, {closure@container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::layout_array::{closure#0}}>` |
| 62 | `<std::collections::TryReserveErrorKind as std::clone::Clone>::clone` |
| 62 | `std::alloc::handle_alloc_error` |
| 62 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::capacity_overflow` |
| 59 | `<std::result::Result<std::alloc::Layout, std::collections::TryReserveError> as std::ops::Try>::branch` |
| 59 | `container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::finish_grow` |
| 59 | `<std::result::Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> as std::ops::Try>::branch` |
| 59 | `std::alloc::Global::grow_impl_runtime` |
| 59 | `std::result::Result::<std::ptr::NonNull<[u8]>, std::alloc::AllocError>::map_err::<std::collections::TryReserveError, {closure@container_decoders_never_panic_on_arbitrary_bytes::{closure#1}::alloc::raw_vec::RawVecInner::finish_grow::{closure#0}}>` |
| 59 | `<std::result::Result<std::ptr::NonNull<[u8]>, std::collections::TryReserveError> as std::ops::Try>::branch` |
