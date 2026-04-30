use super::*;

fn collect_all<T: Copy>(buf: &RingBuffer<T>) -> Vec<T> {
    buf.all().copied().collect()
}

fn test_case(cap: usize, range: Option<(usize, usize)>, expect: &[usize]) {
    let mut buf = RingBuffer::<usize>::new(cap);
    if let Some((start, end)) = range {
        for i in start..=end {
            buf.update(i);
        }
    }

    let all = collect_all(&buf);
    assert_eq!(buf.size(), expect.len());
    assert_eq!(&all, expect);
}

#[test]
fn test_all_empty_buffer() {
    test_case(5, None, &[]);
}

#[test]
fn test_all_partially_full() {
    test_case(5, Some((1, 3)), &[1, 2, 3]);
}

#[test]
fn test_all_full_not_wrapped() {
    test_case(5, Some((1, 5)), &[1, 2, 3, 4, 5]);
}

#[test]
fn test_all_wrapped_once() {
    test_case(5, Some((1, 6)), &[2, 3, 4, 5, 6]);
}

#[test]
fn test_all_wrapped_multiple_times() {
    test_case(5, Some((1, 7)), &[3, 4, 5, 6, 7]);
}

#[test]
fn test_all_wrapped_exceeding_twice_capacity() {
    test_case(3, Some((1, 7)), &[5, 6, 7]);
}

#[test]
fn test_all_capacity_one() {
    let mut buf = RingBuffer::new(1);
    buf.update(1);
    assert_eq!(collect_all(&buf), vec![1]);

    buf.update(2);
    assert_eq!(collect_all(&buf), vec![2]);

    buf.update(3);
    assert_eq!(collect_all(&buf), vec![3]);
}

#[test]
fn test_all_verify_order_with_collect() {
    test_case(4, Some((1, 7)), &[4, 5, 6, 7]);
}
