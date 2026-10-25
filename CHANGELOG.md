# Changelog

## [1.12.3](https://github.com/juninmd/faxina/compare/v1.12.2...v1.12.3) (2026-10-25)


### Bug Fixes

* **remove:** try every entry of a folder even after one fails ([#66](https://github.com/juninmd/faxina/issues/66)) ([e66e552](https://github.com/juninmd/faxina/commit/e66e5526b5b65c6cfdf81818fc94e836148fc2a8))
* **ui:** ignore stale responses when the depth changes ([#64](https://github.com/juninmd/faxina/issues/64)) ([802dcb2](https://github.com/juninmd/faxina/commit/802dcb24bd1bc423258e0fe1cfb6a523d2848e73))


### Performance Improvements

* **ui:** page the empty-folder list instead of rendering every row ([#65](https://github.com/juninmd/faxina/issues/65)) ([78d21d6](https://github.com/juninmd/faxina/commit/78d21d679e0139b15e7443fedb13496c9b4685c9))

## [1.12.2](https://github.com/juninmd/faxina/compare/v1.12.1...v1.12.2) (2026-10-25)


### Bug Fixes

* **a11y:** keep the safety card heading out of an interactive button ([#62](https://github.com/juninmd/faxina/issues/62)) ([7852c20](https://github.com/juninmd/faxina/commit/7852c207bc422144485fe8d0303dfb084b089e8b))
* **guard:** protect more token stores under home from deletion ([#57](https://github.com/juninmd/faxina/issues/57)) ([86fdc0b](https://github.com/juninmd/faxina/commit/86fdc0b96eca860c48411fa7559e89fa0eff6c3e))
* **remove:** re-check that the path is still a link at removal time ([#63](https://github.com/juninmd/faxina/issues/63)) ([d8e6b15](https://github.com/juninmd/faxina/commit/d8e6b15a3ab1331d07869c64a41330e0e71c8f79))
* **ui:** do not report "no stale projects" before the check finishes ([#60](https://github.com/juninmd/faxina/issues/60)) ([19b0bf7](https://github.com/juninmd/faxina/commit/19b0bf77c4d86a57335cbc9d1e1ef835e861a2c7))
* **ui:** drop the junk analysis result when the user cancels it ([#61](https://github.com/juninmd/faxina/issues/61)) ([bb134c7](https://github.com/juninmd/faxina/commit/bb134c700b8a5015d77e18acb301e254b8814514))
* **ui:** report folder-picker and copy-path failures instead of hiding them ([#58](https://github.com/juninmd/faxina/issues/58)) ([048ce86](https://github.com/juninmd/faxina/commit/048ce86bbc7312f3e17b7fc8a10b159e3082ffc9))
* **ui:** stop reporting a failed comparison as the first analysis ([#59](https://github.com/juninmd/faxina/issues/59)) ([2d48f09](https://github.com/juninmd/faxina/commit/2d48f090fdea1bd7dda577892a040f047462f052))


### Performance Improvements

* **dupes:** size the prefix probe buffer to the file ([#56](https://github.com/juninmd/faxina/issues/56)) ([6226c7c](https://github.com/juninmd/faxina/commit/6226c7cf8a858d6d996d039e4a73b60e89aa8c95))

## [1.12.1](https://github.com/juninmd/faxina/compare/v1.12.0...v1.12.1) (2026-10-25)


### Bug Fixes

* **guard:** refuse deletion of credential stores under a scan root ([#33](https://github.com/juninmd/faxina/issues/33)) ([aa7a952](https://github.com/juninmd/faxina/commit/aa7a9528802394c28663d87e7bc5c903e81075ca))

# [1.12.0](https://github.com/juninmd/faxina/compare/v1.11.0...v1.12.0) (2026-10-25)


### Bug Fixes

* **a11y:** expose sunburst controls and treemap mark state to assistive tech ([#38](https://github.com/juninmd/faxina/issues/38)) ([3afaa5f](https://github.com/juninmd/faxina/commit/3afaa5ff784568e0805835e834004a85d3b8c29e))
* **dupes:** drop results when the folder or minimum size changes ([#43](https://github.com/juninmd/faxina/issues/43)) ([fd6973e](https://github.com/juninmd/faxina/commit/fd6973e9bb1f0e186c29b5d7866939200a38aaf3))
* **dupes:** keep the hash cache writing once it passes its cap ([#37](https://github.com/juninmd/faxina/issues/37)) ([902f72d](https://github.com/juninmd/faxina/commit/902f72d1048e6cc04cac6ef2171da1db5c7937f2))
* **empty:** report a link as dangling only when its target is missing ([#40](https://github.com/juninmd/faxina/issues/40)) ([da490c5](https://github.com/juninmd/faxina/commit/da490c5e147255373735eb4493bebdee7124fc42))
* **safety:** reset the permanent-delete choice when the dialog closes or the destination changes ([#41](https://github.com/juninmd/faxina/issues/41)) ([d441b52](https://github.com/juninmd/faxina/commit/d441b525375f0bdebfd34e265ce85818d992fa1b)), closes [#54](https://github.com/juninmd/faxina/issues/54)
* **ui:** stop showing invented disk capacity before the reading arrives ([#42](https://github.com/juninmd/faxina/issues/42)) ([6042ccd](https://github.com/juninmd/faxina/commit/6042ccd8e01b22e846800de17d14cba9eec8d361))
* **ui:** tell the user when copying the assistant prompt fails ([#39](https://github.com/juninmd/faxina/issues/39)) ([35398a9](https://github.com/juninmd/faxina/commit/35398a98d0705320fa74943a246aa7973415c074))


### Features

* **a11y:** move between first-ring sunburst arcs with the arrow keys ([#55](https://github.com/juninmd/faxina/issues/55)) ([1e5e107](https://github.com/juninmd/faxina/commit/1e5e107ec7ed0d37a227a62e41c4253fb6ca5af2))

# [1.11.0](https://github.com/juninmd/faxina/compare/v1.10.1...v1.11.0) (2026-10-25)


### Bug Fixes

* **history:** keep a damaged history file instead of overwriting it ([#51](https://github.com/juninmd/faxina/issues/51)) ([1c2fb8a](https://github.com/juninmd/faxina/commit/1c2fb8a2b541ea95edf9505cd956a7d1f4bdccc9))
* release hole animation rows and stop dupes progress on panic ([#45](https://github.com/juninmd/faxina/issues/45)) ([97a6f8b](https://github.com/juninmd/faxina/commit/97a6f8bfd58eaf2819d4285949d11ad282e78a98))
* **scan:** ignore stale folder responses and report open failures ([#50](https://github.com/juninmd/faxina/issues/50)) ([f36b779](https://github.com/juninmd/faxina/commit/f36b779e9a5d31c740276348e9f162616694a2e3))
* **ui:** clear the report copy timer and report copy failures ([#44](https://github.com/juninmd/faxina/issues/44)) ([812a056](https://github.com/juninmd/faxina/commit/812a056136b8e8ba90571386df2058bcc623fd1d))
* **ui:** keep junk selections when a setting changes ([#48](https://github.com/juninmd/faxina/issues/48)) ([47f85ac](https://github.com/juninmd/faxina/commit/47f85ac0a0b6af6dca0619e32317faae8fa1a820))
* **ui:** reset the collector's confirm state when its list empties ([#49](https://github.com/juninmd/faxina/issues/49)) ([a4c03e7](https://github.com/juninmd/faxina/commit/a4c03e719121c97c4dc73513a5100a51dbf9bfc0))


### Features

* **a11y:** move between top-level treemap tiles with the arrow keys ([#47](https://github.com/juninmd/faxina/issues/47)) ([ab93c63](https://github.com/juninmd/faxina/commit/ab93c63465830a724386771093b854f4d7994af2))
* **dupes:** show the hashing stage and its own total instead of a bare count ([#53](https://github.com/juninmd/faxina/issues/53)) ([48977d0](https://github.com/juninmd/faxina/commit/48977d087377169da376f366d0a723a91e072ac2))


### Performance Improvements

* **empty:** run broken-link removal off the async runtime ([#46](https://github.com/juninmd/faxina/issues/46)) ([fa3cef7](https://github.com/juninmd/faxina/commit/fa3cef7ab76cad500f1bce45db40ae52bc3a9c74))

## [1.10.1](https://github.com/juninmd/faxina/compare/v1.10.0...v1.10.1) (2026-10-25)


### Bug Fixes

* **ui:** honest copy feedback and big-files loading; perf: cheaper scan metadata ([#32](https://github.com/juninmd/faxina/issues/32)) ([47117ed](https://github.com/juninmd/faxina/commit/47117ed2ee5078f62f07fb24fc91775014d47c42))


### Performance Improvements

* **ui:** page the sidebar suggestion and stale-project lists ([#34](https://github.com/juninmd/faxina/issues/34)) ([c4a8c36](https://github.com/juninmd/faxina/commit/c4a8c3697ce639c504797f2b58519381c830be7d))

# [1.10.0](https://github.com/juninmd/faxina/compare/v1.9.0...v1.10.0) (2026-10-25)


### Features

* recognize more build/cache folders and find empty folders and broken links ([#18](https://github.com/juninmd/faxina/issues/18)) ([971ca79](https://github.com/juninmd/faxina/commit/971ca7918629c1fba49a286c2413d1168e5900fc))

# [1.9.0](https://github.com/juninmd/faxina/compare/v1.8.0...v1.9.0) (2026-10-24)


### Features

* add count-up numbers, grain and aurora hero, glowing CTA and staggered reveal ([#16](https://github.com/juninmd/faxina/issues/16)) ([776a4ce](https://github.com/juninmd/faxina/commit/776a4ceac57eec6633900514a9b9e0859c105cf3))
* add count-up numbers, grain and aurora hero, glowing CTA and staggered reveal ([#17](https://github.com/juninmd/faxina/issues/17)) ([5f2bd3f](https://github.com/juninmd/faxina/commit/5f2bd3fbca456aee723afb3281014cc1e16e18b3))

# [1.8.0](https://github.com/juninmd/faxina/compare/v1.7.0...v1.8.0) (2026-10-24)


### Features

* what grew since last scan, folder explanations and idle project cleanup ([#15](https://github.com/juninmd/faxina/issues/15)) ([cfeddc2](https://github.com/juninmd/faxina/commit/cfeddc2a5a88a431c7229de18a005b91d701970e))

# [1.7.0](https://github.com/juninmd/faxina/compare/v1.6.0...v1.7.0) (2026-10-23)


### Features

* list node_modules of projects idle for N months ([#14](https://github.com/juninmd/faxina/issues/14)) ([fe41097](https://github.com/juninmd/faxina/commit/fe410972cf59cfa162b00bbafc468317347ebbb3))

# [1.6.0](https://github.com/juninmd/faxina/compare/v1.5.2...v1.6.0) (2026-10-20)


### Features

* clean more AI, toolchain and app caches, with an opt-in mode ([#11](https://github.com/juninmd/faxina/issues/11)) ([646a51b](https://github.com/juninmd/faxina/commit/646a51b525b4a669d903b4738038fec5c1977ffd))

## [1.5.2](https://github.com/juninmd/faxina/compare/v1.5.1...v1.5.2) (2026-10-18)


### Performance Improvements

* faster cold scans and recognise more dependency and cache folders ([#10](https://github.com/juninmd/faxina/issues/10)) ([e5750f7](https://github.com/juninmd/faxina/commit/e5750f711cba1cb9114e35cb99751012911adbe3))

## [1.5.1](https://github.com/juninmd/faxina/compare/v1.5.0...v1.5.1) (2026-10-17)


### Bug Fixes

* **deps:** upgrade rustls to patch four TLS advisories ([#7](https://github.com/juninmd/faxina/issues/7)) ([e764828](https://github.com/juninmd/faxina/commit/e7648282b025232377de638e715db52e2c921af2))

# [1.5.0](https://github.com/juninmd/faxina/compare/v1.4.0...v1.5.0) (2026-10-17)


### Features

* **cleanup:** make rescans reliable and add a biggest-files view ([3868bc1](https://github.com/juninmd/faxina/commit/3868bc1a63d680a46ec11cbe02338c5f65cf5ced))

# [1.4.0](https://github.com/juninmd/faxina/compare/v1.3.0...v1.4.0) (2026-10-15)


### Features

* integra redesign do prototipo com suporte a scroll e controles nativos ([#5](https://github.com/juninmd/faxina/issues/5)) ([fd43a06](https://github.com/juninmd/faxina/commit/fd43a060d3b1a939188da14c22381b6fe3c96b58))

# [1.3.0](https://github.com/juninmd/faxina/compare/v1.2.1...v1.3.0) (2026-10-09)


### Features

* amplia cobertura de caches e acelera a busca de duplicatas ([#4](https://github.com/juninmd/faxina/issues/4)) ([458caed](https://github.com/juninmd/faxina/commit/458caed3b020fe6d24010777e1bc325d72fd6c7a))

## [1.2.1](https://github.com/juninmd/faxina/compare/v1.2.0...v1.2.1) (2026-09-29)


### Performance Improvements

* batch trash and parallelize permanent deletes ([26e0884](https://github.com/juninmd/faxina/commit/26e0884f60dbf4631088466abdb8cec1b4f844e5))

# [1.2.0](https://github.com/juninmd/faxina/compare/v1.1.0...v1.2.0) (2026-09-27)


### Features

* count hard links once and polish the cleaning flows ([9b04597](https://github.com/juninmd/faxina/commit/9b045973734e55cb6a1e082a5bb46a8f7d9170e9))

# [1.1.0](https://github.com/juninmd/faxina/compare/v1.0.2...v1.1.0) (2026-09-27)


### Features

* choose the drive to scan and harden deletion safety ([730fba5](https://github.com/juninmd/faxina/commit/730fba55036b42f6f1fb60f3e18e5f105f52e511))

## [1.0.2](https://github.com/juninmd/faxina/compare/v1.0.1...v1.0.2) (2026-09-27)


### Bug Fixes

* **ci:** build the updater manifest once from signed release assets ([8833f43](https://github.com/juninmd/faxina/commit/8833f43013222dff134b121962c7aa10932d56c7))

## [1.0.1](https://github.com/juninmd/faxina/compare/v1.0.0...v1.0.1) (2026-09-27)


### Bug Fixes

* **ci:** wait for the draft release before building ([9779a14](https://github.com/juninmd/faxina/commit/9779a141ed5bf0a88ec10236e07a5a13522c5494))

# 1.0.0 (2026-09-27)


### Features

* visual disk cleaner with space map, junk cleaner and duplicate finder ([57978f9](https://github.com/juninmd/faxina/commit/57978f955526e8ca4c5812e65bfc4a9e40fa5a8c))
