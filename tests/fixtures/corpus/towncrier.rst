Release notes
#############

``towncrier`` issues are filed on `GitHub <https://github.com/twisted/towncrier/issues>`_, and each ticket number here corresponds to a closed GitHub issue.

.. towncrier release notes start

Towncrier 24.8.0 (2024-08-23)
=============================

No changes since the previous release candidate.


Features
--------

- Add ``.gitkeep`` as an ignored filename. (`#643 <https://github.com/twisted/towncrier/issues/643>`_)
- Config `ignore` option now supports wildcard matching via `fnmatch <https://docs.python.org/3/library/fnmatch.html#fnmatch.fnmatch>`_. (`#644 <https://github.com/twisted/towncrier/issues/644>`_)
- Add a config for enforcing issue names using regex. (`#649 <https://github.com/twisted/towncrier/issues/649>`_)


Bugfixes
--------

- The template file is now ignored based only on the file name. (`#638 <https://github.com/twisted/towncrier/issues/638>`_)
- Control of the header formatting is once again completely up to the user when they are writing markdown files (fixes a regression introduced in [#610](https://github.com/twisted/towncrier/pull/610)). (`#651 <https://github.com/twisted/towncrier/issues/651>`_)
- Fixed an issue where `issue_template` failed recognizing the issue name of files with a non-category suffix (`.md`) (`#654 <https://github.com/twisted/towncrier/issues/654>`_)
- Fixed a bug where orphan news fragments (e.g. +abc1234.feature) would fail when an `issue_pattern` is configured. Orphan news fragments are now excempt from `issue_pattern` checks. (`#655 <https://github.com/twisted/towncrier/issues/655>`_)


Deprecations and Removals
-------------------------

- Moved towncrier version definition from src/towncrier/_version.py to pyproject.toml

  towncrier.__version__ was removed, after being deprecated in 23.6.0. (`#640 <https://github.com/twisted/towncrier/issues/640>`_)


Misc
----

- `#640 <https://github.com/twisted/towncrier/issues/640>`_, `#657 <https://github.com/twisted/towncrier/issues/657>`_


