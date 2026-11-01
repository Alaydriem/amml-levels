add_rules("mode.debug", "mode.release")

add_repositories("levimc-repo https://github.com/LiteLDev/xmake-repo.git")

add_requires("levilamina 26.51.6", {configs = {target_type = "client"}})
add_requires("levibuildscript 0.6.2")

if not has_config("vs_runtime") then
    set_runtimes("MD")
end

target("levels-spike")
    add_rules("@levibuildscript/linkrule")
    add_rules("@levibuildscript/modpacker", {modName = "levels-spike", modVersion = "0.0.1"})
    add_packages("levilamina")
    add_syslinks("user32", "kernel32")
    add_defines("NOMINMAX", "UNICODE", "_AMD64_", "LL_PLAT_C")
    set_exceptions("none")
    add_cxflags("/EHa", "/utf-8", "/W4")
    add_cxflags(
        "/EHs",
        "-Wno-microsoft-cast",
        "-Wno-invalid-offsetof",
        "-Wno-c++2b-extensions",
        "-Wno-microsoft-include",
        "-Wno-overloaded-virtual",
        "-Wno-ignored-qualifiers",
        "-Wno-missing-field-initializers",
        "-Wno-potentially-evaluated-expression",
        "-Wno-pragma-system-header-outside-header",
        {tools = {"clang_cl"}}
    )
    set_toolchains("clang-cl")
    set_kind("shared")
    set_languages("c++20")
    set_symbols("debug")
    add_files("src/**.cpp")
    add_includedirs("src")
