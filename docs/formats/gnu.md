# GNU NEWS and ChangeLog

Still doing fine with plain text.

## NEWS releases

```sh
logloom parse examples/gnu-news.txt --input-format gnu-news
```

```text
* Noteworthy changes in release 2.0 (2026-11-04)
** Bug fixes

  Preserve Unicode paths.
  Continuation text stays in the same entry.
```

Use `Noteworthy changes in release`, `Changes in version`, `Changes in release`,
or `Version` headings. Outline stars are optional; `**` starts a category.
Blank lines separate prose entries.

The `?.?`/`????-??-??` development placeholders become a pending release with no
version or date. Annotations like `[stable]` stay in the original heading.

## The classic ChangeLog

```sh
logloom parse examples/gnu-ChangeLog --input-format gnu-changelog
```

```text
2026-11-04  Alice Example  <alice@example.com>

	* src/reader.c (read_file): Preserve Unicode paths.
	Keep continuation lines together.
```

An ISO date and author start a record. File/function entries, continuation
indentation, and introductions stay in. `version` is null; attributes include
`author` and `record-kind: "dated-change"`. Multiple authors can share a date.

## Which vintage works

Both readers keep raw text. Git-based version reconstruction and outline rendering
aren't implemented. NEWS needs the release headings listed above; Emacs-specific
topic/version layouts and pre-ISO or localized ChangeLog dates aren't interpreted.
Missing recognized headers get an error. Validation checks calendar dates separately.

Upstream examples: [coreutils NEWS](https://github.com/coreutils/coreutils/blob/master/NEWS),
[traditional Emacs ChangeLog](https://github.com/emacs-mirror/emacs/blob/master/ChangeLog.1).
