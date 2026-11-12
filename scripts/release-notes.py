#!/usr/bin/env python3
"""Validate release identity and extract its reviewed changelog entry. No Git writes."""
import argparse
from datetime import date
import json
from pathlib import Path
import re
import sys
import tomllib

VERSION = re.compile(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?')


def valid_version(value):
    match = VERSION.fullmatch(value)
    if not match or len(value) > 63:
        raise ValueError('Use MAJOR.MINOR.PATCH or MAJOR.MINOR.PATCH-rc.N without build metadata')
    if match[4] and any(part.isdecimal() and len(part) > 1 and part.startswith('0') for part in match[4].split('.')):
        raise ValueError('Numeric prerelease identifiers cannot have leading zeroes')
    return match[4] is not None


def changelog_entries(text):
    headings = list(re.finditer(r'^## (.+)$', text, re.M))
    entries = {}
    for index, heading in enumerate(headings):
        match = re.fullmatch(r'\[([^\]]+)\](?: - ([0-9]{4}-[0-9]{2}-[0-9]{2}))?', heading[1])
        if not match:
            raise ValueError('Changelog headings must be [Unreleased] or [VERSION] - YYYY-MM-DD')
        version, released = match.groups()
        if version in entries:
            raise ValueError('Duplicate changelog version')
        body = text[heading.end():headings[index + 1].start() if index + 1 < len(headings) else len(text)].strip()
        if version == 'Unreleased':
            if index != 0 or released is not None:
                raise ValueError('Keep undated [Unreleased] first')
        else:
            valid_version(version)
            if released is None:
                raise ValueError('Versioned changelog entries need a release date')
            date.fromisoformat(released)
            if not re.search(r'^- \S', body, re.M):
                raise ValueError('A release needs at least one user-visible changelog bullet')
        entries[version] = {'date': released, 'body': body}
    if 'Unreleased' not in entries:
        raise ValueError('Keep an [Unreleased] changelog section')
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--check', action='store_true')
    mode.add_argument('--tag')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--source-root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    try:
        root = args.source_root
        version = tomllib.loads((root / 'frontend/Cargo.toml').read_text())['package']['version']
        valid_version(version)
        entries = changelog_entries((root / 'CHANGELOG.md').read_text())
        if args.check:
            if args.output:
                parser.error('--output requires --tag')
            print('Changelog structure and application version OK')
            return
        if not args.output:
            parser.error('--tag requires --output')
        if not args.tag.startswith('v'):
            raise ValueError('Release tags must start with v')
        tagged = args.tag[1:]
        prerelease = valid_version(tagged)
        if tagged != version:
            raise ValueError('Tag must match frontend/Cargo.toml package.version exactly')
        if tagged not in entries:
            raise ValueError('Tag needs its own dated CHANGELOG.md entry; prepare a release commit first')
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(entries[tagged]['body'] + '\n')
        print(json.dumps({'tag': args.tag, 'version': version, 'prerelease': prerelease,
                          'date': entries[tagged]['date']}))
    except (ValueError, KeyError, OSError, TypeError) as error:
        print(f'Release metadata: {error}', file=sys.stderr)
        sys.exit(2)


if __name__ == '__main__':
    main()
