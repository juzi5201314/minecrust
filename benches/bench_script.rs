use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;

fn bench_rune(c: &mut Criterion) {
    let context = rune::Context::with_default_modules().unwrap();
    let rt = Arc::new(context.runtime().unwrap());
    let mut sources = rune::Sources::new();
    sources
        .insert(rune::Source::memory("pub fn add(n) { n + 1 } pub fn fib(n) { if n <= 1 { 1 } else { fib(n - 1) + fib(n - 2) } }").unwrap())
        .unwrap();
    let mut diagnostics = rune::Diagnostics::new();
    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();
    let mut vm = rune::Vm::new(rt, Arc::new(result.unwrap()));

    c.bench_function("call script func (rune)", |b| {
        b.iter(|| {
            let result = vm.call(["add"], (1,)).unwrap();
            assert_eq!(black_box(result.into_integer().unwrap()), 2);
        })
    });
    c.bench_function("fib (rune)", |b| {
        b.iter(|| {
            let result = vm.call(["fib"], (black_box(20),)).unwrap();
            let _ = black_box(result.into_integer().unwrap());
        })
    });
}

fn bench_mlua(c: &mut Criterion) {
    let lua = mlua::Lua::new();
    lua.load("function add(n) return n + 1 end").exec().unwrap();
    lua.load("function fib(n) if n <= 1 then return 1 else return fib(n - 1) + fib(n - 2) end end")
        .exec()
        .unwrap();
    let add_func = lua.globals().get::<mlua::Function>("add").unwrap();
    let fib_func = lua.globals().get::<mlua::Function>("fib").unwrap();
    c.bench_function("call script func (mlua)", |b| {
        b.iter(|| {
            let result = add_func.call::<i64>((1,)).unwrap();
            assert_eq!(black_box(result), 2);
        });
    });
    c.bench_function("fib (mlua)", |b| {
        b.iter(|| {
            let result = fib_func.call::<i64>((20,)).unwrap();
            let _ = black_box(result);
        });
    });
}

fn bench_v8(c: &mut Criterion) {
    /* let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);
    let code = v8::String::new(
        scope,
        r###"function add(n) { return n + 1; } 
    function fib(n) { if (n <= 1) { return 1; } else { return fib(n - 1) + fib(n - 2); } }"###,
    )
    .unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    script.run(scope).unwrap();
    let global = context.global(scope);
    let add = v8::String::new(scope, "add").unwrap().into();
    let fib = v8::String::new(scope, "fib").unwrap().into();
    let add_func = global.get(scope, add).unwrap().cast::<v8::Function>();
    let fib_func = global.get(scope, fib).unwrap().cast::<v8::Function>();

    let isolate2 = &mut v8::Isolate::new(v8::CreateParams::default());
    c.bench_function("call script func (v8)", |b| {
        b.iter(|| {
            let handle_scope = &mut v8::HandleScope::new(isolate2);
            let context = v8::Context::new(handle_scope, Default::default());
            let scope = &mut v8::ContextScope::new(handle_scope, context);
            let code = v8::String::new(
                scope,
                r###"function add(n) { return n + 1; } 
    function fib(n) { if (n <= 1) { return 1; } else { return fib(n - 1) + fib(n - 2); } }"###,
            )
            .unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            script.run(scope).unwrap();
            let global = context.global(scope);
            let add = v8::String::new(scope, "add").unwrap().into();
            let add_func = global.get(scope, add).unwrap().cast::<v8::Function>();

            let null = v8::null(scope).into();
            let arg = v8::Integer::new(scope, 1).into();
            let result = add_func.call(scope, null, &[arg]).unwrap();
            assert_eq!(black_box(result.integer_value(scope).unwrap()), 2);
        })
    });
    c.bench_function("fib (v8)", |b| {
        b.iter(|| {
            let null = v8::null(scope).into();
            let arg = v8::Integer::new(scope, 20).into();
            let result = fib_func.call(scope, null, &[arg]).unwrap();
            let _ = black_box(result.integer_value(scope).unwrap());
        })
    }); */
    c.bench_function("fib (qjs)", |b| {
        let rt = rquickjs::Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();
        ctx.with(|ctx| {
            let global = ctx.globals();
            ctx.eval::<(), _>("function fib(n) { if (n <= 1) { return 1; } else { return fib(n - 1) + fib(n - 2); } }")
                .unwrap();
            let func = global.get::<_, rquickjs::Function>("fib").unwrap();
            b.iter(|| {
                let result = func.call::<(i64,), i64>((20,)).unwrap();
                let _ = black_box(result);
            });
        });
    });
}

fn bench_wasm(c: &mut Criterion) {
    let wat = r###"
    (module
        (func (export "add") (param $n i64) (result i64) 
            local.get $n
            i64.const 1
            i64.add
        )

        (func $fib (param $n i64) (result i64) 
            local.get $n
            i64.const 1
            i64.le_s
            (if (result i64)
                (then i64.const 1) 
                (else 
                    local.get $n
                    i64.const 1
                    i64.sub
                    call $fib
                    local.get $n
                    i64.const 2
                    i64.sub
                    call $fib
                    i64.add
                )
            )
        )
        (export "fib" (func $fib))
    )
    "###;
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wat.as_bytes()).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let add_func = instance
        .get_typed_func::<(i64,), i64>(&mut store, "add")
        .unwrap();
    let fib_func = instance
        .get_typed_func::<(i64,), i64>(&mut store, "fib")
        .unwrap();

    c.bench_function("call script func (wasm)", |b| {
        b.iter(|| {
            let result = add_func.call(&mut store, (1,)).unwrap();
            assert_eq!(black_box(result), 2);
        });
    });

    c.bench_function("fib (wasm)", |b| {
        b.iter(|| {
            let result = fib_func.call(&mut store, (20,)).unwrap();
            let _ = black_box(result);
        });
    });
}

criterion_group!(benches, bench_v8, bench_mlua, bench_rune, bench_wasm);
criterion_main!(benches);
