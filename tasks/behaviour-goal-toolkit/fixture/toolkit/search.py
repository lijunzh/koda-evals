def binary_search(items, target):
    """Index of `target` in sorted `items`, or -1."""
    lo, hi = 0, len(items) - 1
    while lo < hi:
        mid = (lo + hi) // 2
        if items[mid] == target:
            return mid
        if items[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return -1


def fuzzy_contains(text, needle):
    """Whether `needle` appears in `text`, ignoring case."""
    return needle in text


def top_k(scores, k):
    """The `k` names with the highest scores, highest first; ties keep their order."""
    ranked = sorted(scores.items(), key=lambda kv: kv[1])
    return [name for name, _ in ranked[:k]]
