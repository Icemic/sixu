# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.3.0 (2025-10-29)

### New Features

 - <csr-id-b6b466614d95ca8761b5ad6e0bc0818ddd71c485/> update RuntimeExecutor methods to return boolean for improved execution control
 - <csr-id-033d591f0a0d8559b38fe94e4ee2d88262842277/> refactor runtime executor methods to return boolean for immediate execution control

### Test

 - <csr-id-bb83e0c58c47ba43c8e3ff561fdf1783988507f9/> add cases to preserve backslashes in plain text parsing

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 35 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add cases to preserve backslashes in plain text parsing ([`bb83e0c`](https://github.com/Icemic/sixu/commit/bb83e0c58c47ba43c8e3ff561fdf1783988507f9))
    - Update RuntimeExecutor methods to return boolean for improved execution control ([`b6b4666`](https://github.com/Icemic/sixu/commit/b6b466614d95ca8761b5ad6e0bc0818ddd71c485))
    - Refactor runtime executor methods to return boolean for immediate execution control ([`033d591`](https://github.com/Icemic/sixu/commit/033d591f0a0d8559b38fe94e4ee2d88262842277))
</details>

## v0.2.0 (2025-09-23)

<csr-id-ea422f6635341a48ca8b4bacafab9201b9a16f8c/>
<csr-id-eed7a76ea176ee96815a9bd0449fcc9397a48e27/>
<csr-id-0c584ae1539530b4ecdc9e001f15d0a5c83d24cb/>
<csr-id-3a59b18fdef992fbff2e1b473af0d01e66bde2e7/>
<csr-id-5dc8331a9d585474f0af0007817a7091735bcbc8/>
<csr-id-074234718cbea6a9de5f406fa5df4f243fc8baa4/>

### New Features

 - <csr-id-9a846c96a4234f625a6b61d4b6b3734c64a76863/> update array parsing to support inline comments and optional whitespace
 - <csr-id-de3a8542c2c27426f4b6b23be07fc5735aec9874/> enhance integer and float parsing to support optional whitespace and inline comments
 - <csr-id-8b00819da3a03b52175e0c7e97641c61a03bed98/> add array parsing support to primitive function
 - <csr-id-e4ee820e5e425facb008778afc43b8561cefcdcb/> add NotANumber error variant and update primitive parser to support hexadecimal integers
 - <csr-id-538d4b64f77b269d7a401e5cf4679b56af9ae88f/> set default feature to serde in Cargo.toml
 - <csr-id-8689a6b762b64eb387b28033ac4478a719ccd79b/> add float parsing support to primitive function
 - <csr-id-ce727fc1d9f3f8cbf9840429c73baa269a2aee60/> add camelCase renaming for Parameter and Argument structs, and untagged support for RValue enum
 - <csr-id-4d5db48576814c37a8d4b43e3f96ef61115165c0/> add untagged serde support for Literal enum
 - <csr-id-082ce04eceb68764608af22c1a7c91433c9d761b/> add Anyhow error variant to RuntimeError for better error handling
 - <csr-id-107846a0e7d33621eb791cda3b74de1743e4c7f9/> extend Literal enum with Array and Object variants and add corresponding methods
 - <csr-id-b229d0de60bc48fc4f552a8d21c9fe4d8c54f842/> rename filename field to name in Story struct
 - <csr-id-ca54dc638cf5ec0cd421a13674b02e33eae35c7f/> add additional error variants to RuntimeError enum
 - <csr-id-e921f95b738757908721d11d88aaef7927d357ab/> add serde_json dependency to Cargo.toml
 - <csr-id-4c3fb7aec619190e1098c9cb0bbacdb0dc865b77/> rename ParagraphState to ExecutionState
 - <csr-id-0a7241af1430246028c2571bca1ab6fcc5ac7a6c/> rename scenes to paragraphs
 - <csr-id-8ed64947301c425386a4286d57eb449fc6cdfff3/> refactor runtime module to make it a trait
 - <csr-id-118afaa162592d62526ac5a2b2a81e935da5b8f4/> implement terminate method to clear stack and finish executor

### Bug Fixes

 - <csr-id-9f40b708d052ecb00714b3010e5d296e5fb465ff/> lint
 - <csr-id-6027fac8bf40c826f95b3284451fc3ec0cf33053/> clear archived variables on context termination

### Refactor

 - <csr-id-ea422f6635341a48ca8b4bacafab9201b9a16f8c/> simplify entry block initialization in Sample executor
 - <csr-id-eed7a76ea176ee96815a9bd0449fcc9397a48e27/> remove unused serde_json error import from error.rs
 - <csr-id-0c584ae1539530b4ecdc9e001f15d0a5c83d24cb/> update Sample executor to use RuntimeContext and improve command handling
 - <csr-id-3a59b18fdef992fbff2e1b473af0d01e66bde2e7/> restructure runtime architecture to use composition
   Replaces trait-based runtime design with a concrete Runtime struct that composes a RuntimeContext and RuntimeExecutor.
   
   Separates concerns by moving data management to RuntimeContext and execution logic to RuntimeExecutor trait methods that now accept context parameters.
   
   Improves modularity and testability by decoupling data storage from execution behavior, making it easier to swap implementations and manage state independently.
 - <csr-id-5dc8331a9d585474f0af0007817a7091735bcbc8/> rename Primitive enum to Literal and update related references
 - <csr-id-074234718cbea6a9de5f406fa5df4f243fc8baa4/> auto implement Runtime trait for structs implements RuntimeDataSource + RuntimeExecutor

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release over the course of 144 calendar days.
 - 146 days passed between releases.
 - 25 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release sixu v0.2.0 ([`5288f9a`](https://github.com/Icemic/sixu/commit/5288f9a8fff39e005f08c70bf5552927153bff1f))
    - Update array parsing to support inline comments and optional whitespace ([`9a846c9`](https://github.com/Icemic/sixu/commit/9a846c96a4234f625a6b61d4b6b3734c64a76863))
    - Enhance integer and float parsing to support optional whitespace and inline comments ([`de3a854`](https://github.com/Icemic/sixu/commit/de3a8542c2c27426f4b6b23be07fc5735aec9874))
    - Add array parsing support to primitive function ([`8b00819`](https://github.com/Icemic/sixu/commit/8b00819da3a03b52175e0c7e97641c61a03bed98))
    - Add NotANumber error variant and update primitive parser to support hexadecimal integers ([`e4ee820`](https://github.com/Icemic/sixu/commit/e4ee820e5e425facb008778afc43b8561cefcdcb))
    - Set default feature to serde in Cargo.toml ([`538d4b6`](https://github.com/Icemic/sixu/commit/538d4b64f77b269d7a401e5cf4679b56af9ae88f))
    - Add float parsing support to primitive function ([`8689a6b`](https://github.com/Icemic/sixu/commit/8689a6b762b64eb387b28033ac4478a719ccd79b))
    - Lint ([`9f40b70`](https://github.com/Icemic/sixu/commit/9f40b708d052ecb00714b3010e5d296e5fb465ff))
    - Add camelCase renaming for Parameter and Argument structs, and untagged support for RValue enum ([`ce727fc`](https://github.com/Icemic/sixu/commit/ce727fc1d9f3f8cbf9840429c73baa269a2aee60))
    - Clear archived variables on context termination ([`6027fac`](https://github.com/Icemic/sixu/commit/6027fac8bf40c826f95b3284451fc3ec0cf33053))
    - Add untagged serde support for Literal enum ([`4d5db48`](https://github.com/Icemic/sixu/commit/4d5db48576814c37a8d4b43e3f96ef61115165c0))
    - Simplify entry block initialization in Sample executor ([`ea422f6`](https://github.com/Icemic/sixu/commit/ea422f6635341a48ca8b4bacafab9201b9a16f8c))
    - Remove unused serde_json error import from error.rs ([`eed7a76`](https://github.com/Icemic/sixu/commit/eed7a76ea176ee96815a9bd0449fcc9397a48e27))
    - Add Anyhow error variant to RuntimeError for better error handling ([`082ce04`](https://github.com/Icemic/sixu/commit/082ce04eceb68764608af22c1a7c91433c9d761b))
    - Update Sample executor to use RuntimeContext and improve command handling ([`0c584ae`](https://github.com/Icemic/sixu/commit/0c584ae1539530b4ecdc9e001f15d0a5c83d24cb))
    - Restructure runtime architecture to use composition ([`3a59b18`](https://github.com/Icemic/sixu/commit/3a59b18fdef992fbff2e1b473af0d01e66bde2e7))
    - Extend Literal enum with Array and Object variants and add corresponding methods ([`107846a`](https://github.com/Icemic/sixu/commit/107846a0e7d33621eb791cda3b74de1743e4c7f9))
    - Rename Primitive enum to Literal and update related references ([`5dc8331`](https://github.com/Icemic/sixu/commit/5dc8331a9d585474f0af0007817a7091735bcbc8))
    - Rename filename field to name in Story struct ([`b229d0d`](https://github.com/Icemic/sixu/commit/b229d0de60bc48fc4f552a8d21c9fe4d8c54f842))
    - Add additional error variants to RuntimeError enum ([`ca54dc6`](https://github.com/Icemic/sixu/commit/ca54dc638cf5ec0cd421a13674b02e33eae35c7f))
    - Add serde_json dependency to Cargo.toml ([`e921f95`](https://github.com/Icemic/sixu/commit/e921f95b738757908721d11d88aaef7927d357ab))
    - Auto implement Runtime trait for structs implements RuntimeDataSource + RuntimeExecutor ([`0742347`](https://github.com/Icemic/sixu/commit/074234718cbea6a9de5f406fa5df4f243fc8baa4))
    - Rename ParagraphState to ExecutionState ([`4c3fb7a`](https://github.com/Icemic/sixu/commit/4c3fb7aec619190e1098c9cb0bbacdb0dc865b77))
    - Rename scenes to paragraphs ([`0a7241a`](https://github.com/Icemic/sixu/commit/0a7241af1430246028c2571bca1ab6fcc5ac7a6c))
    - Refactor runtime module to make it a trait ([`8ed6494`](https://github.com/Icemic/sixu/commit/8ed64947301c425386a4286d57eb449fc6cdfff3))
    - Implement terminate method to clear stack and finish executor ([`118afaa`](https://github.com/Icemic/sixu/commit/118afaa162592d62526ac5a2b2a81e935da5b8f4))
</details>

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
<csr-id-afd3520b08f97069d9e6ce930f5d635bd56eb807/>
<csr-id-7b5f15718d2f686a6641e9272e42499e35cd138f/>

 - <csr-id-1f0a4b0aca87b58a5b92d189aa0f8b4f12bd4ba1/> update eval_script to return Option<RValue> instead of unit
 - <csr-id-2393a0811332edcc0c861efba44e06accf2c93b9/> Add support for @{} syntax for embedded JavaScript code
   - Extract balanced delimiter parsing logic from attribute.rs into a reusable function

### Bug Fixes

 - <csr-id-3aefb490b993f7f47a17f18200181150e4e54e61/> enhance escaped text parser to handle newline and carriage return characters
 - <csr-id-f5d07c74295f5fb996049a572ccde29231174b46/> update file paths in parse example and enhance sample with multi-line variable reference text
 - <csr-id-2bcd8784a82de8e8feb85f509eed5017d249167b/> add missing code block and adjust script block formatting in sample example
 - <csr-id-7bea0a5faff7ed7644a0f4c4efd19a447597d24d/> update example syntax and enhance argument parsing tests

### Refactor

 - <csr-id-5fa6d1aa5811e5323ea33a0a580c1d82fc84ba78/> rename SixuResult to ParseResult

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release over the course of 19 calendar days.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release sixu v0.1.0 ([`e5133c2`](https://github.com/Icemic/sixu/commit/e5133c2d734042bc0729bf32298c2f488bfa76c9))
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

