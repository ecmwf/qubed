import argparse
import sys

from rich.console import Console

from qubed import Qube
from qubed.convert import parse_fdb_list

console = Console(stderr=True)


def main():
    parser = argparse.ArgumentParser(description="Generate a compressed tree from various inputs.")
    
    subparsers = parser.add_subparsers(title="subcommands", required=True)
    parser_convert = subparsers.add_parser('convert', help='Convert trees from one format to another.')
    parser_another = subparsers.add_parser('another_subcommand', help='Does something else')

    parser_convert.add_argument(
        "--input",
        type=argparse.FileType("r"),
        default=sys.stdin,
        help="Specify the input file (default: standard input)."
    )
    parser_convert.add_argument(
        "--output",
        type=argparse.FileType("w"),
        default=sys.stdout,
        help="Specify the output file (default: standard output)."
    )

    parser_convert.add_argument(
        "--input_format",
        choices=["fdb", "mars"],
        default="fdb",
        help="""Specify the input format:
            fdb: the output of fdb list --porcelain
            mars: the output of mars list
        """
    )
    
    parser_convert.add_argument(
        "--output_format",
        choices=["text", "html"],
        default="text",
        help="Specify the output format (text or html)."
    )
    parser_convert.set_defaults(func=convert)
    
    args = parser.parse_args()
    args.func(args)

def convert(args):
    q = Qube.empty()
    for datacube in parse_fdb_list(args.input):
        new_branch = Qube.from_datacube(datacube)
        q = (q | Qube.from_datacube(datacube))

    # output = match args.output_format:
    #     case "text":
    #         str(q)
    #     case "html":
    #         q.html()
    output = "fw"

    with open(args.output, "w") as f:
        f.write(output)

    console.print([1, 2, 3])
    console.print("[blue underline]Looks like a link")
    console.print(locals())
    console.print("FOO", style="white on blue")

if __name__ == "__main__":
    main()
