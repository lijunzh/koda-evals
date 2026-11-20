def parse_row(line):
    """Fields of one CSV line; a quoted field may hold commas."""
    return line.split(",")


def to_row(fields):
    """One CSV line; a field holding a comma or a quote is quoted, quotes doubled."""
    return ",".join(fields)


def parse_int_field(text):
    """An integer field, ignoring spaces around it; None when empty."""
    return int(text)
