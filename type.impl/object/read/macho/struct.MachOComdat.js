(function() {
    var type_impls = Object.fromEntries([["object",[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-MachOComdat%3C'data,+'file,+Mach,+R%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#560\">Source</a><a href=\"#impl-Debug-for-MachOComdat%3C'data,+'file,+Mach,+R%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'data, 'file, Mach, R&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"object/read/macho/struct.MachOComdat.html\" title=\"struct object::read::macho::MachOComdat\">MachOComdat</a>&lt;'data, 'file, Mach, R&gt;<div class=\"where\">where\n    Mach: <a class=\"trait\" href=\"object/read/macho/trait.MachHeader.html\" title=\"trait object::read::macho::MachHeader\">MachHeader</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,\n    R: <a class=\"trait\" href=\"object/read/trait.ReadRef.html\" title=\"trait object::read::ReadRef\">ReadRef</a>&lt;'data&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#560\">Source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/1.84.1/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.84.1/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","object::read::macho::file::MachOComdat32","object::read::macho::file::MachOComdat64"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-ObjectComdat%3C'data%3E-for-MachOComdat%3C'data,+'file,+Mach,+R%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#577-608\">Source</a><a href=\"#impl-ObjectComdat%3C'data%3E-for-MachOComdat%3C'data,+'file,+Mach,+R%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'data, 'file, Mach, R&gt; <a class=\"trait\" href=\"object/read/trait.ObjectComdat.html\" title=\"trait object::read::ObjectComdat\">ObjectComdat</a>&lt;'data&gt; for <a class=\"struct\" href=\"object/read/macho/struct.MachOComdat.html\" title=\"struct object::read::macho::MachOComdat\">MachOComdat</a>&lt;'data, 'file, Mach, R&gt;<div class=\"where\">where\n    Mach: <a class=\"trait\" href=\"object/read/macho/trait.MachHeader.html\" title=\"trait object::read::macho::MachHeader\">MachHeader</a>,\n    R: <a class=\"trait\" href=\"object/read/trait.ReadRef.html\" title=\"trait object::read::ReadRef\">ReadRef</a>&lt;'data&gt;,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.SectionIterator\" class=\"associatedtype trait-impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#582\">Source</a><a href=\"#associatedtype.SectionIterator\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"object/read/trait.ObjectComdat.html#associatedtype.SectionIterator\" class=\"associatedtype\">SectionIterator</a> = <a class=\"struct\" href=\"object/read/macho/struct.MachOComdatSectionIterator.html\" title=\"struct object::read::macho::MachOComdatSectionIterator\">MachOComdatSectionIterator</a>&lt;'data, 'file, Mach, R&gt;</h4></section></summary><div class='docblock'>An iterator for the sections in the section group.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.kind\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#585-587\">Source</a><a href=\"#method.kind\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"object/read/trait.ObjectComdat.html#tymethod.kind\" class=\"fn\">kind</a>(&amp;self) -&gt; <a class=\"enum\" href=\"object/read/enum.ComdatKind.html\" title=\"enum object::read::ComdatKind\">ComdatKind</a></h4></section></summary><div class='docblock'>Returns the COMDAT selection kind.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.symbol\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#590-592\">Source</a><a href=\"#method.symbol\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"object/read/trait.ObjectComdat.html#tymethod.symbol\" class=\"fn\">symbol</a>(&amp;self) -&gt; <a class=\"struct\" href=\"object/read/struct.SymbolIndex.html\" title=\"struct object::read::SymbolIndex\">SymbolIndex</a></h4></section></summary><div class='docblock'>Returns the index of the symbol used for the name of COMDAT section group.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.name_bytes\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#595-597\">Source</a><a href=\"#method.name_bytes\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"object/read/trait.ObjectComdat.html#tymethod.name_bytes\" class=\"fn\">name_bytes</a>(&amp;self) -&gt; <a class=\"type\" href=\"object/read/type.Result.html\" title=\"type object::read::Result\">Result</a>&lt;&amp;'data [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.u8.html\">u8</a>]&gt;</h4></section></summary><div class='docblock'>Returns the name of the COMDAT section group.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.name\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#600-602\">Source</a><a href=\"#method.name\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"object/read/trait.ObjectComdat.html#tymethod.name\" class=\"fn\">name</a>(&amp;self) -&gt; <a class=\"type\" href=\"object/read/type.Result.html\" title=\"type object::read::Result\">Result</a>&lt;&amp;'data <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.84.1/std/primitive.str.html\">str</a>&gt;</h4></section></summary><div class='docblock'>Returns the name of the COMDAT section group. <a href=\"object/read/trait.ObjectComdat.html#tymethod.name\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.sections\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/object/read/macho/file.rs.html#605-607\">Source</a><a href=\"#method.sections\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"object/read/trait.ObjectComdat.html#tymethod.sections\" class=\"fn\">sections</a>(&amp;self) -&gt; Self::<a class=\"associatedtype\" href=\"object/read/trait.ObjectComdat.html#associatedtype.SectionIterator\" title=\"type object::read::ObjectComdat::SectionIterator\">SectionIterator</a></h4></section></summary><div class='docblock'>Get the sections in this section group.</div></details></div></details>","ObjectComdat<'data>","object::read::macho::file::MachOComdat32","object::read::macho::file::MachOComdat64"]]]]);
    if (window.register_type_impls) {
        window.register_type_impls(type_impls);
    } else {
        window.pending_type_impls = type_impls;
    }
})()
//{"start":55,"fragment_lengths":[7706]}