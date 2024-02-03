use std::{env, fs, process::Command};

use cranelift::prelude::{
    settings, AbiParam, Configurable, FunctionBuilder, FunctionBuilderContext, InstBuilder,
    Signature, Variable,
};
use cranelift_module::{DataDescription, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

fn main() {
    let bytes = generate_code();

    fs::write("my_awesome_program.o", bytes.as_slice()).unwrap();
    println!("created my_awesome_program.o\n");

    if let Some(arg) = env::args().nth(1) {
        if arg == "with_bug" {
            link(true);
        } else if arg == "without_bug" {
            link(false)
        } else {
            println!("only `with_bug` and `without_bug` are accepted");
        }
    }
}

// ld -platform_version macos 14.2 14.2 -syslibroot /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk -lSystem -ld_classic -o my_awesome_program  my_awesome_program.o
fn link(should_fail: bool) {
    let status = Command::new("ld")
        .args([
            //"-arch",
            //"arm64",
            "-platform_version",
            "macos",
            "14.2",
            "14.2",
            "-syslibroot",
            "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk",
            "-lSystem",
            if should_fail {
                "-ld_new"
            } else {
                "-ld_classic"
            },
            "-o",
            "my_awesome_program",
            "my_awesome_program.o",
        ])
        .status()
        .expect("failed to execute 'ld'");

    println!();
    if !status.success() {
        println!("Linker failure (the bug)");
    } else if should_fail {
        println!("Wow you fixed it :)");
    } else {
        println!("Linker success");
    }
}

fn generate_code() -> Vec<u8> {
    let mut flag_builder = settings::builder();
    // flag_builder.set("use_colocated_libcalls", "false").unwrap();
    flag_builder.set("is_pic", "true").unwrap();

    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .unwrap();

    let builder = ObjectBuilder::new(
        isa,
        "my_awesome_program",
        cranelift_module::default_libcall_names(),
    )
    .unwrap();
    let mut module = ObjectModule::new(builder);

    let mut builder_context = FunctionBuilderContext::new();
    let mut ctx = module.make_context();

    let ptr_ty = module.target_config().pointer_type();

    // generate main function

    let cmain_sig = Signature {
        params: vec![AbiParam::new(ptr_ty), AbiParam::new(ptr_ty)],
        returns: vec![AbiParam::new(ptr_ty)],
        call_conv: module.target_config().default_call_conv,
    };
    let cmain_id = module
        .declare_function("main", Linkage::Export, &cmain_sig)
        .unwrap();

    ctx.func.signature = cmain_sig;

    // Create the builder to build a function.
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_context);

    // Create the entry block, to start emitting code in.
    let entry_block = builder.create_block();

    builder.switch_to_block(entry_block);
    // tell the builder that the block will have no further predecessors
    builder.seal_block(entry_block);

    let arg_argc = builder.append_block_param(entry_block, ptr_ty);
    let arg_argv = builder.append_block_param(entry_block, ptr_ty);

    let var_argc = Variable::from_u32(0);
    builder.declare_var(var_argc, ptr_ty);
    builder.def_var(var_argc, arg_argc);

    let var_argv = Variable::from_u32(1);
    builder.declare_var(var_argv, ptr_ty);
    builder.def_var(var_argv, arg_argv);

    generate_hello_world(&mut module, &mut builder);

    let exit_code = builder.ins().iconst(ptr_ty, 42);
    builder.ins().return_(&[exit_code]);

    builder.seal_all_blocks();
    builder.finalize();

    // optional debuging
    // println!("main\n{}", ctx.func);

    module
        .define_function(cmain_id, &mut ctx)
        .expect("error defining function");

    module.clear_context(&mut ctx);

    // now finalize everything.

    let product = module.finish();

    product.emit().unwrap()
}

fn generate_hello_world(module: &mut ObjectModule, builder: &mut FunctionBuilder) {
    let ptr_ty = module.target_config().pointer_type();

    // generate the signature for the libc "puts" function
    let puts = {
        let puts_sig = Signature {
            params: vec![AbiParam::new(ptr_ty)],
            returns: vec![],
            call_conv: module.target_config().default_call_conv,
        };

        let id = module
            .declare_function("puts", Linkage::Import, &puts_sig)
            .unwrap();

        module.declare_func_in_func(id, builder.func)
    };

    // create a global string to print
    let string_val = {
        let text = "Hello, World!\0";

        let mut data_desc = DataDescription::new();
        data_desc.define(text.as_bytes().to_vec().into_boxed_slice());

        let id = module
            .declare_data(".str", Linkage::Local, false, false)
            .unwrap();

        module.define_data(id, &data_desc).unwrap();

        let global = module.declare_data_in_func(id, builder.func);

        // gets a pointer to the global data
        builder.ins().global_value(ptr_ty, global)
    };

    builder.ins().call(puts, &[string_val]);
}
