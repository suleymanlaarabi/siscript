#![forbid(unsafe_code)]

use si_ast::item::{FunctionKind, ItemKind};
use si_ast::ty::TypeKind;
use si_bytecode::{Constant, Instruction};

use crate::locals::LocalMap;
use crate::lowering::Compiler;

impl Compiler<'_> {
    pub(crate) fn compile_script_functions(&mut self) {
        let items: Vec<_> = self.checked.typed.ast.items.clone();
        for item in &items {
            match &item.kind {
                ItemKind::Function(function) => {
                    if function.kind == FunctionKind::Extern {
                        continue;
                    }
                    self.compile_function_item(function);
                }
                ItemKind::Struct(item) => {
                    for method in &item.methods {
                        self.compile_function_item(method);
                    }
                }
                _ => {}
            }
        }
    }

    fn compile_function_item(&mut self, function: &si_ast::item::FunctionItem) {
        let Some(id) = self.functions.get(&function.name).copied() else { return };
        let mut locals = LocalMap::default();
        locals.enter();
        for param in &function.params {
            locals.insert(&param.name);
        }
        if let Some(body) = &function.body {
            self.compile_block(id, &mut locals, body, true);
        }
        let function_bc = self.builder.function_mut(id);
        if !matches!(function_bc.instructions.last(), Some(Instruction::Return)) {
            if function.return_ty.as_ref().is_none_or(|ty| matches!(ty.kind, TypeKind::Void)) {
                let idx = function_bc.push_const(Constant::Void);
                function_bc.emit(Instruction::Const(idx));
            }
            function_bc.emit(Instruction::Return);
        }
        function_bc.locals_count = locals.count();
    }
}
