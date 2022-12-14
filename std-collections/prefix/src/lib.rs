#![forbid(unsafe_code)]

pub fn longest_common_prefix(strs: Vec<&str>) -> String {
    // len returns number of bytes and works as expected for the unicode strings
    let min_len = strs.iter().map(|x| x.len()).min().unwrap_or(0);

    if min_len == 0 {
        String::new()
    } else {
        let potential = strs[0].clone();
        let mut iters = strs
            .into_iter()
            .map(|x| x.char_indices())
            .collect::<Vec<_>>();
        for _ in 0..min_len {
            let mut cur = &mut iters[0].next().unwrap().clone();
            for s in &mut iters[1..] {
                let act = s.next().unwrap();
                if *cur != act {
                    return potential[0..cur.0].to_string();
                }
            }
        }
        return potential[0..min_len].to_string();
    }
}
