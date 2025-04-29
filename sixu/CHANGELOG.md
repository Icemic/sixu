# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2025-04-30)

<csr-id-5fa6d1aa5811e5323ea33a0a580c1d82fc84ba78/>

### New Features

<csr-id-f39c6c33fe800c0a92866f6028314af6cf68d2f5/>
<csr-id-c6939e2f22469f07010162ee1177fcf0946b419c/>
<csr-id-1c43f417b4790c2de2086292040c0d0fc1ecaf64/>
<csr-id-243dac2b3e307e4e669d4a2db6fd95b6346ccac4/>
<csr-id-b50144da92fd9fda31e29120f181608c951f75a9/>
<csr-id-c3aaa4357906da38f1c35bfbc361c217d748f2ae/>
<csr-id-591ee274b2124c1ea54e34edd5b4dde3043f3ef9/>
<csr-id-6ca5c539af48570cd7ec4cf26e377b6181120079/>
<csr-id-97245e070f6f1c770b023fb9b0713b5d34a99332/>

 - <csr-id-1f0a4b0aca87b58a5b92d189aa0f8b4f12bd4ba1/> update eval_script to return Option<RValue> instead of unit
 - <csr-id-2393a0811332edcc0c861efba44e06accf2c93b9/> Add support for @{} syntax for embedded JavaScript code
   - Extract balanced delimiter parsing logic from attribute.rs into a reusable function
- Add support for @{} syntax for embedded JavaScript code while maintaining compatibility with ## syntax
- Update syntax documentation to reflect new embedded code syntax options
- Add comprehensive tests for the new syntax
 - <csr-id-afd3520b08f97069d9e6ce930f5d635bd56eb807/> update Cargo.toml with project metadata and add README.md
 - <csr-id-7b5f15718d2f686a6641e9272e42499e35cd138f/> create CHANGELOG.md to document project updates and versioning

### Bug Fixes

 - <csr-id-3aefb490b993f7f47a17f18200181150e4e54e61/> enhance escaped text parser to handle newline and carriage return characters
 - <csr-id-f5d07c74295f5fb996049a572ccde29231174b46/> update file paths in parse example and enhance sample with multi-line variable reference text
 - <csr-id-2bcd8784a82de8e8feb85f509eed5017d249167b/> add missing code block and adjust script block formatting in sample example
 - <csr-id-7bea0a5faff7ed7644a0f4c4efd19a447597d24d/> update example syntax and enhance argument parsing tests

### Refactor

 - <csr-id-5fa6d1aa5811e5323ea33a0a580c1d82fc84ba78/> rename SixuResult to ParseResult

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release over the course of 19 calendar days.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update Cargo.toml with project metadata and add README.md ([`afd3520`](https://github.com/Icemic/sixu/commit/afd3520b08f97069d9e6ce930f5d635bd56eb807))
    - Create CHANGELOG.md to document project updates and versioning ([`7b5f157`](https://github.com/Icemic/sixu/commit/7b5f15718d2f686a6641e9272e42499e35cd138f))
    - Update eval_script to return Option<RValue> instead of unit ([`1f0a4b0`](https://github.com/Icemic/sixu/commit/1f0a4b0aca87b58a5b92d189aa0f8b4f12bd4ba1))
    - Add support for @{} syntax for embedded JavaScript code ([`2393a08`](https://github.com/Icemic/sixu/commit/2393a0811332edcc0c861efba44e06accf2c93b9))
    - Implement attribute parsing  and update block structure to support attributes ([`f39c6c3`](https://github.com/Icemic/sixu/commit/f39c6c33fe800c0a92866f6028314af6cf68d2f5))
    - Add `#break` call ([`c6939e2`](https://github.com/Icemic/sixu/commit/c6939e2f22469f07010162ee1177fcf0946b419c))
    - Add serde support; implement save and restore methods in Runtime ([`1c43f41`](https://github.com/Icemic/sixu/commit/1c43f417b4790c2de2086292040c0d0fc1ecaf64))
    - Refactor: runtime; tests: add runtime test case; ([`7c502ee`](https://github.com/Icemic/sixu/commit/7c502ee7b4e6779251880e2d0cdf697e2ba8f38b))
    - Refactor text and template parsing to use unified Text enum for ChildContent ([`243dac2`](https://github.com/Icemic/sixu/commit/243dac2b3e307e4e669d4a2db6fd95b6346ccac4))
    - Refactor TemplateLiteral structure to use parts instead of separate strings and values ([`b50144d`](https://github.com/Icemic/sixu/commit/b50144da92fd9fda31e29120f181608c951f75a9))
    - Rename SixuResult to ParseResult ([`5fa6d1a`](https://github.com/Icemic/sixu/commit/5fa6d1aa5811e5323ea33a0a580c1d82fc84ba78))
    - Enhance escaped text parser to handle newline and carriage return characters ([`3aefb49`](https://github.com/Icemic/sixu/commit/3aefb490b993f7f47a17f18200181150e4e54e61))
    - Upgrade nom to v0.8 ([`c3aaa43`](https://github.com/Icemic/sixu/commit/c3aaa4357906da38f1c35bfbc361c217d748f2ae))
    - Update file paths in parse example and enhance sample with multi-line variable reference text ([`f5d07c7`](https://github.com/Icemic/sixu/commit/f5d07c74295f5fb996049a572ccde29231174b46))
    - Improve leading text and add template literal support in parser and format ([`591ee27`](https://github.com/Icemic/sixu/commit/591ee274b2124c1ea54e34edd5b4dde3043f3ef9))
    - Add leading text support in syntax and parser ([`6ca5c53`](https://github.com/Icemic/sixu/commit/6ca5c539af48570cd7ec4cf26e377b6181120079))
    - Add missing code block and adjust script block formatting in sample example ([`2bcd878`](https://github.com/Icemic/sixu/commit/2bcd8784a82de8e8feb85f509eed5017d249167b))
    - Update example syntax and enhance argument parsing tests ([`7bea0a5`](https://github.com/Icemic/sixu/commit/7bea0a5faff7ed7644a0f4c4efd19a447597d24d))
    - Move to monorepo ([`97245e0`](https://github.com/Icemic/sixu/commit/97245e070f6f1c770b023fb9b0713b5d34a99332))
</details>

<csr-unknown>
 implement attribute parsing  and update block structure to support attributes add #break call add serde support; implement save and restore methods in Runtime refactor text and template parsing to use unified Text enum for ChildContent refactor TemplateLiteral structure to use parts instead of separate strings and values upgrade nom to v0.8 improve leading text and add template literal support in parser and format add leading text support in syntax and parser move to monorepo<csr-unknown/>

