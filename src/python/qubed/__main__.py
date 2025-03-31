import json
import time

import click
import psutil
from rich.console import Console
from rich.layout import Layout
from rich.live import Live
from rich.panel import Panel
from rich.spinner import Spinner
from rich.text import Text

from qubed import Qube
from qubed.convert import parse_fdb_list

console = Console(stderr=True)
process = psutil.Process()

PRINT_INTERVAL = 0.25


@click.group()
def main():
    """Command-line tool for working with trees."""
    pass


@main.command()
@click.option(
    "--input",
    type=click.File("r"),
    default="-",
    help="Specify the input file (default: standard input).",
)
@click.option(
    "--output",
    type=click.File("w"),
    default="-",
    help="Specify the output file (default: standard output).",
)
@click.option(
    "--from",
    "from_format",
    type=click.Choice(["fdb", "mars"]),
    default="fdb",
    help="Specify the input format: fdb (fdb list --porcelain) or mars (mars list).",
)
@click.option(
    "--to",
    "to_format",
    type=click.Choice(["text", "html", "json"]),
    default="text",
    help="Specify the output format: text, html, json.",
)
def convert(input, output, from_format, to_format):
    """Convert trees from one format to another."""
    q = Qube.empty()
    t = time.time()
    i0 = 0
    n0 = 0
    depth = 5
    log = Text()
    summary = Layout()
    summary.split_column(
        Layout(name="upper"),
        Layout(name="qube"),
    )
    summary["upper"].split_row(
        Layout(name="performance"),
        Layout(log, name="log"),
    )
    spinner = Spinner("aesthetic", text="Performance", speed=0.3)

    with Live(summary, auto_refresh=False, transient=True, console=console) as live:
        for i, datacube in enumerate(parse_fdb_list(input)):
            new_branch = Qube.from_datacube(datacube)
            q = q | new_branch

            if time.time() - t > PRINT_INTERVAL:
                tree = q.__str__(depth=depth)
                if tree.count("\n") > 20:
                    depth -= 1
                if tree.count("\n") < 5:
                    depth += 1

                summary["performance"].update(
                    Panel(
                        Text.assemble(
                            f"The Qube has {q.n_leaves} leaves and {q.n_nodes} internal nodes so far.\n",
                            f"{(i - i0) / (time.time() - t) / PRINT_INTERVAL:.0f} lines per second.  ",
                            f"{(q.n_leaves - n0) / (time.time() - t):.0f} leaves per second.\n",
                            f"Memory usage: {process.memory_info().rss / 1024 / 1024:.0f} MB\n",
                        ),
                        title=spinner.render(time.time()),
                        border_style="blue",
                    )
                )
                summary["qube"].update(
                    Panel(tree, title=f"Qube (depth {depth})", border_style="blue")
                )
                summary["log"].update(
                    Panel(
                        f"{datacube}", border_style="blue", title="Last Datacube Added"
                    )
                )
                live.refresh()
                i0 = i
                n0 = q.n_leaves
                t = time.time()

    if to_format == "text":
        output_content = str(q)
    elif to_format == "json":
        output_content = json.dumps(q.to_json())
    elif to_format == "html":
        output_content = q.html().html
    else:
        output_content = str(q)

    output.write(output_content)


if __name__ == "__main__":
    main()
