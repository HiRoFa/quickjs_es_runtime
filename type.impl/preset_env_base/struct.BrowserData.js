(function() {
    var type_impls = Object.fromEntries([["preset_env_base",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BrowserData%3COption%3CVersion%3E%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#69-123\">Source</a><a href=\"#impl-BrowserData%3COption%3CVersion%3E%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"preset_env_base/version/struct.Version.html\" title=\"struct preset_env_base::version::Version\">Version</a>&gt;&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.is_any_target\" class=\"method\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#71-73\">Source</a><h4 class=\"code-header\">pub fn <a href=\"preset_env_base/struct.BrowserData.html#tymethod.is_any_target\" class=\"fn\">is_any_target</a>(&amp;self) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class=\"docblock\"><p>Returns true if all fields are <a href=\"https://doc.rust-lang.org/1.84.1/core/option/enum.Option.html#variant.None\" title=\"variant core::option::Option::None\">None</a>.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.parse_versions\" class=\"method\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#76-122\">Source</a><h4 class=\"code-header\">pub fn <a href=\"preset_env_base/struct.BrowserData.html#tymethod.parse_versions\" class=\"fn\">parse_versions</a>(distribs: <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"struct\" href=\"browserslist/queries/struct.Distrib.html\" title=\"struct browserslist::queries::Distrib\">Distrib</a>&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Self, <a class=\"struct\" href=\"anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Parses the value returned from <code>browserslist</code> as <a href=\"preset_env_base/type.Versions.html\" title=\"type preset_env_base::Versions\">Versions</a>.</p>\n</div></details></div></details>",0,"preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><section id=\"method.iter\" class=\"method\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><h4 class=\"code-header\">pub fn <a href=\"preset_env_base/struct.BrowserData.html#tymethod.iter\" class=\"fn\">iter</a>(&amp;self) -&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserDataRefIter.html\" title=\"struct preset_env_base::BrowserDataRefIter\">BrowserDataRefIter</a>&lt;'_, T&gt; <a href=\"#\" class=\"tooltip\" data-notable-ty=\"BrowserDataRefIter&lt;&#39;_, T&gt;\">ⓘ</a></h4></section></div></details>",0,"preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><section id=\"method.iter_mut\" class=\"method\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><h4 class=\"code-header\">pub fn <a href=\"preset_env_base/struct.BrowserData.html#tymethod.iter_mut\" class=\"fn\">iter_mut</a>(&amp;mut self) -&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserDataMutIter.html\" title=\"struct preset_env_base::BrowserDataMutIter\">BrowserDataMutIter</a>&lt;'_, T&gt; <a href=\"#\" class=\"tooltip\" data-notable-ty=\"BrowserDataMutIter&lt;&#39;_, T&gt;\">ⓘ</a></h4></section></div></details>",0,"preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><section id=\"method.map\" class=\"method\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><h4 class=\"code-header\">pub fn <a href=\"preset_env_base/struct.BrowserData.html#tymethod.map\" class=\"fn\">map</a>&lt;N: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt;(\n    self,\n    op: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/function/trait.FnMut.html\" title=\"trait core::ops::function::FnMut\">FnMut</a>(&amp;'static <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.str.html\">str</a>, T) -&gt; N,\n) -&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;N&gt;</h4></section><section id=\"method.map_value\" class=\"method\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><h4 class=\"code-header\">pub fn <a href=\"preset_env_base/struct.BrowserData.html#tymethod.map_value\" class=\"fn\">map_value</a>&lt;N: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt;(self, op: impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/function/trait.FnMut.html\" title=\"trait core::ops::function::FnMut\">FnMut</a>(T) -&gt; N) -&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;N&gt;</h4></section></div></details>",0,"preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Clone-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.84.1/src/core/clone.rs.html#174\">Source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: &amp;Self)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.84.1/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Debug-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Default-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Deserialize%3C'de%3E-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de, T&gt; <a class=\"trait\" href=\"serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"serde/de/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;__D&gt;(__deserializer: __D) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;Self, __D::<a class=\"associatedtype\" href=\"serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __D: <a class=\"trait\" href=\"serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</div></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"serde/de/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Index%3C%26K%3E-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Index%3C%26K%3E-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'a, T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>, K: ?<a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/borrow/trait.Borrow.html\" title=\"trait core::borrow::Borrow\">Borrow</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.str.html\">str</a>&gt;&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/index/trait.Index.html\" title=\"trait core::ops::index::Index\">Index</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.reference.html\">&amp;'a K</a>&gt; for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Output\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#associatedtype.Output\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"https://doc.rust-lang.org/1.84.1/core/ops/index/trait.Index.html#associatedtype.Output\" class=\"associatedtype\">Output</a> = T</h4></section></summary><div class='docblock'>The returned type after indexing.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.index\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.index\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/ops/index/trait.Index.html#tymethod.index\" class=\"fn\">index</a>(&amp;self, v: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.reference.html\">&amp;K</a>) -&gt; &amp;Self::<a class=\"associatedtype\" href=\"https://doc.rust-lang.org/1.84.1/core/ops/index/trait.Index.html#associatedtype.Output\" title=\"type core::ops::index::Index::Output\">Output</a></h4></section></summary><div class='docblock'>Performs the indexing (<code>container[index]</code>) operation. <a href=\"https://doc.rust-lang.org/1.84.1/core/ops/index/trait.Index.html#tymethod.index\">Read more</a></div></details></div></details>","Index<&'a K>","preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-IntoIterator-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-IntoIterator-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/iter/traits/collect/trait.IntoIterator.html\" title=\"trait core::iter::traits::collect::IntoIterator\">IntoIterator</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Item\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#associatedtype.Item\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"https://doc.rust-lang.org/1.84.1/core/iter/traits/collect/trait.IntoIterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = (&amp;'static <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.str.html\">str</a>, T)</h4></section></summary><div class='docblock'>The type of the elements being iterated over.</div></details><details class=\"toggle\" open><summary><section id=\"associatedtype.IntoIter\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#associatedtype.IntoIter\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"https://doc.rust-lang.org/1.84.1/core/iter/traits/collect/trait.IntoIterator.html#associatedtype.IntoIter\" class=\"associatedtype\">IntoIter</a> = <a class=\"struct\" href=\"arrayvec/arrayvec/struct.IntoIter.html\" title=\"struct arrayvec::arrayvec::IntoIter\">IntoIter</a>&lt;(&amp;'static <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.str.html\">str</a>, T), 21&gt;</h4></section></summary><div class='docblock'>Which kind of iterator are we turning this into?</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.into_iter\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.into_iter\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/iter/traits/collect/trait.IntoIterator.html#tymethod.into_iter\" class=\"fn\">into_iter</a>(self) -&gt; Self::<a class=\"associatedtype\" href=\"https://doc.rust-lang.org/1.84.1/core/iter/traits/collect/trait.IntoIterator.html#associatedtype.IntoIter\" title=\"type core::iter::traits::collect::IntoIterator::IntoIter\">IntoIter</a></h4></section></summary><div class='docblock'>Creates an iterator from a value. <a href=\"https://doc.rust-lang.org/1.84.1/core/iter/traits/collect/trait.IntoIterator.html#tymethod.into_iter\">Read more</a></div></details></div></details>","IntoIterator","preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-PartialEq-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>self</code> and <code>other</code> values to be equal, and is used by <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.84.1/src/core/cmp.rs.html#261\">Source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>Tests for <code>!=</code>. The default implementation is almost always sufficient,\nand should not be overridden without very good reason.</div></details></div></details>","PartialEq","preset_env_base::Versions"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Serialize-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T&gt; <a class=\"trait\" href=\"serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;__S&gt;(&amp;self, __serializer: __S) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.84.1/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;__S::<a class=\"associatedtype\" href=\"serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, __S::<a class=\"associatedtype\" href=\"serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<div class=\"where\">where\n    __S: <a class=\"trait\" href=\"serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</div></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","preset_env_base::Versions"],["<section id=\"impl-Copy-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Copy-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section>","Copy","preset_env_base::Versions"],["<section id=\"impl-Eq-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-Eq-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section>","Eq","preset_env_base::Versions"],["<section id=\"impl-StructuralPartialEq-for-BrowserData%3CT%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/preset_env_base/lib.rs.html#16\">Source</a><a href=\"#impl-StructuralPartialEq-for-BrowserData%3CT%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/marker/trait.StructuralPartialEq.html\" title=\"trait core::marker::StructuralPartialEq\">StructuralPartialEq</a> for <a class=\"struct\" href=\"preset_env_base/struct.BrowserData.html\" title=\"struct preset_env_base::BrowserData\">BrowserData</a>&lt;T&gt;</h3></section>","StructuralPartialEq","preset_env_base::Versions"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[29343]}