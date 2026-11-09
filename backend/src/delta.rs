//! Line-oriented unified review delta compatible with Python difflib's
//! no-junk SequenceMatcher for the bounded text packet contract.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Match {
    a: usize,
    b: usize,
    len: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    Equal,
    Replace,
    Delete,
    Insert,
}
#[derive(Clone, Copy, Debug)]
struct Op {
    tag: Tag,
    a0: usize,
    a1: usize,
    b0: usize,
    b1: usize,
}

fn longest(
    a: &[&str],
    b: &[&str],
    a0: usize,
    a1: usize,
    b0: usize,
    b1: usize,
    positions: &BTreeMap<&str, Vec<usize>>,
) -> Match {
    let mut best = Match {
        a: a0,
        b: b0,
        len: 0,
    };
    let mut prev = BTreeMap::<usize, usize>::new();
    for (i, line) in a.iter().enumerate().take(a1).skip(a0) {
        let mut current = BTreeMap::<usize, usize>::new();
        if let Some(indices) = positions.get(line) {
            for &j in indices {
                if j < b0 {
                    continue;
                }
                if j >= b1 {
                    break;
                }
                let len = if j > 0 {
                    prev.get(&(j - 1)).copied().unwrap_or(0)
                } else {
                    0
                } + 1;
                current.insert(j, len);
                if len > best.len {
                    best = Match {
                        a: i + 1 - len,
                        b: j + 1 - len,
                        len,
                    };
                }
            }
        }
        prev = current;
    }
    // Difflib extends the non-junk match beyond auto-popular anchors.
    while best.a > a0 && best.b > b0 && a[best.a - 1] == b[best.b - 1] {
        best.a -= 1;
        best.b -= 1;
        best.len += 1;
    }
    while best.a + best.len < a1
        && best.b + best.len < b1
        && a[best.a + best.len] == b[best.b + best.len]
    {
        best.len += 1;
    }
    best
}
fn matching(a: &[&str], b: &[&str]) -> Vec<Match> {
    let mut positions = BTreeMap::<&str, Vec<usize>>::new();
    for (j, line) in b.iter().enumerate() {
        positions.entry(line).or_default().push(j);
    }
    if b.len() >= 200 {
        let threshold = b.len() / 100 + 1;
        positions.retain(|_, v| v.len() <= threshold);
    }
    let mut queue = vec![(0, a.len(), 0, b.len())];
    let mut matches = Vec::new();
    while let Some((a0, a1, b0, b1)) = queue.pop() {
        let m = longest(a, b, a0, a1, b0, b1, &positions);
        if m.len == 0 {
            continue;
        }
        if a0 < m.a && b0 < m.b {
            queue.push((a0, m.a, b0, m.b));
        }
        if m.a + m.len < a1 && m.b + m.len < b1 {
            queue.push((m.a + m.len, a1, m.b + m.len, b1));
        }
        matches.push(m);
    }
    matches.sort_by_key(|m| (m.a, m.b));
    let mut collapsed: Vec<Match> = Vec::new();
    for m in matches {
        if let Some(last) = collapsed.last_mut()
            && last.a + last.len == m.a
            && last.b + last.len == m.b
        {
            last.len += m.len;
            continue;
        }
        collapsed.push(m);
    }
    collapsed.push(Match {
        a: a.len(),
        b: b.len(),
        len: 0,
    });
    collapsed
}
fn opcodes(a: &[&str], b: &[&str]) -> Vec<Op> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    for m in matching(a, b) {
        let tag = if i < m.a && j < m.b {
            Some(Tag::Replace)
        } else if i < m.a {
            Some(Tag::Delete)
        } else if j < m.b {
            Some(Tag::Insert)
        } else {
            None
        };
        if let Some(tag) = tag {
            out.push(Op {
                tag,
                a0: i,
                a1: m.a,
                b0: j,
                b1: m.b,
            });
        }
        if m.len > 0 {
            out.push(Op {
                tag: Tag::Equal,
                a0: m.a,
                a1: m.a + m.len,
                b0: m.b,
                b1: m.b + m.len,
            });
        }
        i = m.a + m.len;
        j = m.b + m.len;
    }
    out
}
fn range(start: usize, stop: usize) -> String {
    let len = stop - start;
    if len == 1 {
        format!("{}", start + 1)
    } else if len == 0 {
        format!("{start},0")
    } else {
        format!("{},{}", start + 1, len)
    }
}
fn lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split_inclusive('\n').collect()
    }
}
pub fn unified(previous: &str, current: &str) -> String {
    let a = lines(previous);
    let b = lines(current);
    let mut ops = opcodes(&a, &b);
    if ops.is_empty() || ops.iter().all(|op| op.tag == Tag::Equal) {
        return String::new();
    }
    let n = 3;
    if ops[0].tag == Tag::Equal {
        let len = ops[0].a1 - ops[0].a0;
        if len > n {
            ops[0].a0 = ops[0].a1 - n;
            ops[0].b0 = ops[0].b1 - n;
        }
    }
    let last = ops.len() - 1;
    if ops[last].tag == Tag::Equal {
        let len = ops[last].a1 - ops[last].a0;
        if len > n {
            ops[last].a1 = ops[last].a0 + n;
            ops[last].b1 = ops[last].b0 + n;
        }
    }
    let mut groups = Vec::<Vec<Op>>::new();
    let mut group = Vec::new();
    for op in ops {
        if op.tag == Tag::Equal && op.a1 - op.a0 > 2 * n {
            group.push(Op {
                tag: Tag::Equal,
                a0: op.a0,
                a1: op.a0 + n,
                b0: op.b0,
                b1: op.b0 + n,
            });
            groups.push(std::mem::take(&mut group));
            group.push(Op {
                tag: Tag::Equal,
                a0: op.a1 - n,
                a1: op.a1,
                b0: op.b1 - n,
                b1: op.b1,
            });
        } else {
            group.push(op);
        }
    }
    if !group.is_empty() && group.iter().any(|op| op.tag != Tag::Equal) {
        groups.push(group);
    }
    let mut output = String::new();
    if !groups.is_empty() {
        output.push_str("--- previous-reviewed\n+++ current-revision\n");
    }
    for group in groups {
        let start = group.first().unwrap();
        let end = group.last().unwrap();
        output.push_str(&format!(
            "@@ -{} +{} @@\n",
            range(start.a0, end.a1),
            range(start.b0, end.b1)
        ));
        for op in group {
            if matches!(op.tag, Tag::Replace | Tag::Delete) {
                for line in &a[op.a0..op.a1] {
                    output.push('-');
                    output.push_str(line);
                }
            }
            if matches!(op.tag, Tag::Replace | Tag::Insert) {
                for line in &b[op.b0..op.b1] {
                    output.push('+');
                    output.push_str(line);
                }
            }
            if op.tag == Tag::Equal {
                for line in &a[op.a0..op.a1] {
                    output.push(' ');
                    output.push_str(line);
                }
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_change_is_empty() {
        assert!(unified("same\n", "same\n").is_empty());
    }
    #[test]
    fn no_final_newline_is_preserved() {
        assert_eq!(
            unified("a\nb", "a\nc"),
            "--- previous-reviewed\n+++ current-revision\n@@ -1,2 +1,2 @@\n a\n-b+c"
        );
    }
}
