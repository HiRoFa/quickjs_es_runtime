(function() {
    var type_impls = Object.fromEntries([["swc",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-CachingResolver%3CR%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#26-28\">source</a><a href=\"#impl-CachingResolver%3CR%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;R&gt; <a class=\"struct\" href=\"swc_ecma_loader/resolvers/lru/struct.CachingResolver.html\" title=\"struct swc_ecma_loader::resolvers::lru::CachingResolver\">CachingResolver</a>&lt;R&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"swc_ecma_loader/resolve/trait.Resolve.html\" title=\"trait swc_ecma_loader::resolve::Resolve\">Resolve</a>,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#30\">source</a><h4 class=\"code-header\">pub fn <a href=\"swc_ecma_loader/resolvers/lru/struct.CachingResolver.html#tymethod.new\" class=\"fn\">new</a>(cap: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.83.0/std/primitive.usize.html\">usize</a>, inner: R) -&gt; <a class=\"struct\" href=\"swc_ecma_loader/resolvers/lru/struct.CachingResolver.html\" title=\"struct swc_ecma_loader::resolvers::lru::CachingResolver\">CachingResolver</a>&lt;R&gt;</h4></section></div></details>",0,"swc::resolver::NodeResolver"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-CachingResolver%3CR%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#8\">source</a><a href=\"#impl-Debug-for-CachingResolver%3CR%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;R&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.83.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"swc_ecma_loader/resolvers/lru/struct.CachingResolver.html\" title=\"struct swc_ecma_loader::resolvers::lru::CachingResolver\">CachingResolver</a>&lt;R&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.83.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"swc_ecma_loader/resolve/trait.Resolve.html\" title=\"trait swc_ecma_loader::resolve::Resolve\">Resolve</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#8\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.83.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.83.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.83.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.83.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.83.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.83.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","swc::resolver::NodeResolver"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Default-for-CachingResolver%3CR%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#17-19\">source</a><a href=\"#impl-Default-for-CachingResolver%3CR%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;R&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.83.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a> for <a class=\"struct\" href=\"swc_ecma_loader/resolvers/lru/struct.CachingResolver.html\" title=\"struct swc_ecma_loader::resolvers::lru::CachingResolver\">CachingResolver</a>&lt;R&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"swc_ecma_loader/resolve/trait.Resolve.html\" title=\"trait swc_ecma_loader::resolve::Resolve\">Resolve</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.83.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.default\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#21\">source</a><a href=\"#method.default\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.83.0/core/default/trait.Default.html#tymethod.default\" class=\"fn\">default</a>() -&gt; <a class=\"struct\" href=\"swc_ecma_loader/resolvers/lru/struct.CachingResolver.html\" title=\"struct swc_ecma_loader::resolvers::lru::CachingResolver\">CachingResolver</a>&lt;R&gt;</h4></section></summary><div class='docblock'>Returns the “default value” for a type. <a href=\"https://doc.rust-lang.org/1.83.0/core/default/trait.Default.html#tymethod.default\">Read more</a></div></details></div></details>","Default","swc::resolver::NodeResolver"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Resolve-for-CachingResolver%3CR%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#38-40\">source</a><a href=\"#impl-Resolve-for-CachingResolver%3CR%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;R&gt; <a class=\"trait\" href=\"swc_ecma_loader/resolve/trait.Resolve.html\" title=\"trait swc_ecma_loader::resolve::Resolve\">Resolve</a> for <a class=\"struct\" href=\"swc_ecma_loader/resolvers/lru/struct.CachingResolver.html\" title=\"struct swc_ecma_loader::resolvers::lru::CachingResolver\">CachingResolver</a>&lt;R&gt;<div class=\"where\">where\n    R: <a class=\"trait\" href=\"swc_ecma_loader/resolve/trait.Resolve.html\" title=\"trait swc_ecma_loader::resolve::Resolve\">Resolve</a>,</div></h3></section></summary><div class=\"impl-items\"><section id=\"method.resolve\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/swc_ecma_loader/resolvers/lru.rs.html#42\">source</a><a href=\"#method.resolve\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"swc_ecma_loader/resolve/trait.Resolve.html#tymethod.resolve\" class=\"fn\">resolve</a>(&amp;self, base: &amp;<a class=\"enum\" href=\"swc_common/syntax_pos/enum.FileName.html\" title=\"enum swc_common::syntax_pos::FileName\">FileName</a>, src: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.83.0/std/primitive.str.html\">str</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.83.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"enum\" href=\"swc_common/syntax_pos/enum.FileName.html\" title=\"enum swc_common::syntax_pos::FileName\">FileName</a>, <a class=\"struct\" href=\"anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt;</h4></section></div></details>","Resolve","swc::resolver::NodeResolver"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[7408]}