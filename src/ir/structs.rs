use crate::ir::core::{ValueAdapter, ValueKind, ValueRc};
use crate::ir::types::{Type, TypeKind};
use crate::utils::NewWithRef;
use intrusive_collections::{intrusive_adapter, LinkedList, LinkedListLink};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};
use std::slice;

/// Represents a program.
pub struct Program {
  vars: LinkedList<ValueAdapter>,
  funcs: LinkedList<FunctionAdapter>,
}

impl Program {
  /// Creates a new IR program.
  pub fn new() -> Self {
    Self {
      vars: LinkedList::default(),
      funcs: LinkedList::default(),
    }
  }

  /// Gets the global variables.
  pub fn vars(&self) -> &LinkedList<ValueAdapter> {
    &self.vars
  }

  /// Gets the function definitions.
  pub fn funcs(&self) -> &LinkedList<FunctionAdapter> {
    &self.funcs
  }

  /// Adds the specific global variable to the current program.
  pub fn add_var(&mut self, value: ValueRc) {
    debug_assert!(
      matches!(value.kind(), ValueKind::GlobalAlloc(..)),
      "`value` must be a global allocation!"
    );
    self.vars.push_back(value);
  }

  /// Adds the specific function to the current program.
  pub fn add_func(&mut self, func: FunctionRc) {
    self.funcs.push_back(func);
  }
}

impl Default for Program {
  fn default() -> Self {
    Self::new()
  }
}

/// Represents a function.
pub struct Function {
  link: LinkedListLink,
  ty: Type,
  name: String,
  params: Vec<ValueRc>,
  inner: RefCell<FunctionInner>,
}

intrusive_adapter! {
  pub FunctionAdapter = FunctionRc: Function { link: LinkedListLink }
}

/// Rc of `Function`.
///
/// Used when a type has ownership of `Function`.
pub type FunctionRc = Rc<Function>;

/// Ref of `Function`.
///
/// Used when a type only needs to refer to `Function`.
pub type FunctionRef = Weak<Function>;

impl Function {
  /// Creates a new function definition.
  pub fn new(name: String, params: Vec<ValueRc>, ret_ty: Type) -> FunctionRc {
    let ty = Type::get_function(
      params
        .iter()
        .map(|p| {
          let ty = p.ty().clone();
          debug_assert!(
            !matches!(ty.kind(), TypeKind::Unit),
            "parameter type must not be `unit`!"
          );
          ty
        })
        .collect(),
      ret_ty,
    );
    Rc::new(Self {
      link: LinkedListLink::new(),
      ty,
      name,
      params,
      inner: RefCell::new(FunctionInner {
        bbs: LinkedList::default(),
      }),
    })
  }

  /// Creates a new function declaration.
  pub fn new_decl(name: String, ty: Type) -> FunctionRc {
    match ty.kind() {
      TypeKind::Function(params, _) => {
        debug_assert!(
          params.iter().all(|p| !matches!(p.kind(), TypeKind::Unit)),
          "parameter type must not be `unit`!"
        )
      }
      _ => panic!("expected a function type!"),
    };
    Rc::new(Self {
      link: LinkedListLink::new(),
      ty,
      name,
      params: Vec::new(),
      inner: RefCell::new(FunctionInner {
        bbs: LinkedList::default(),
      }),
    })
  }

  /// Gets the type of the current function.
  pub fn ty(&self) -> &Type {
    &self.ty
  }

  /// Gets the function name.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Gets the parameter list.
  pub fn params(&self) -> &[ValueRc] {
    &self.params
  }

  /// Immutably borrows the inner of the current function.
  ///
  /// # Panics
  ///
  /// Panics if the inner function is currently mutably borrowed.
  pub fn inner(&self) -> Ref<FunctionInner> {
    self.inner.borrow()
  }

  /// Mutably borrows the inner of the current function.
  ///
  /// # Panics
  ///
  /// Panics if the inner function is currently borrowed.
  pub fn inner_mut(&self) -> RefMut<FunctionInner> {
    self.inner.borrow_mut()
  }
}

pub struct FunctionInner {
  bbs: LinkedList<BasicBlockAdapter>,
}

impl FunctionInner {
  /// Gets the basic block list.
  ///
  /// If `bbs` is empty, the current function will be a declaration.
  /// Otherwise, the first basic block in the list will be the entry.
  pub fn bbs(&self) -> &LinkedList<BasicBlockAdapter> {
    &self.bbs
  }

  /// Gets the mutable basic block list.
  pub fn bbs_mut(&mut self) -> &mut LinkedList<BasicBlockAdapter> {
    &mut self.bbs
  }

  /// Adds the specific basic block to the current function.
  pub fn add_bb(&mut self, bb: BasicBlockRc) {
    self.bbs.push_back(bb);
  }
}

/// Represents a basic block.
pub struct BasicBlock {
  link: LinkedListLink,
  name: Option<String>,
  inner: RefCell<BasicBlockInner>,
}

intrusive_adapter! {
  pub BasicBlockAdapter = BasicBlockRc: BasicBlock { link: LinkedListLink }
}

/// Rc of `BasicBlock`.
///
/// Used when a type has ownership of `BasicBlock`.
pub type BasicBlockRc = Rc<BasicBlock>;

/// Ref of `BasicBlock`.
///
/// Used when a type only needs to refer to `BasicBlock`.
pub type BasicBlockRef = Weak<BasicBlock>;

impl BasicBlock {
  /// Creates a new basic block.
  pub fn new(name: Option<String>) -> BasicBlockRc {
    Rc::new_with_ref(|bb| Self {
      link: LinkedListLink::new(),
      name,
      inner: RefCell::new(BasicBlockInner {
        bb,
        preds: Vec::new(),
        insts: LinkedList::default(),
      }),
    })
  }

  /// Gets the name.
  pub fn name(&self) -> &Option<String> {
    &self.name
  }

  /// Immutably borrows the inner of the current basic block.
  ///
  /// # Panics
  ///
  /// Panics if the inner basic block is currently mutably borrowed.
  pub fn inner(&self) -> Ref<BasicBlockInner> {
    self.inner.borrow()
  }

  /// Mutably borrows the inner of the current basic block.
  ///
  /// # Panics
  ///
  /// Panics if the inner basic block is currently borrowed.
  pub fn inner_mut(&self) -> RefMut<BasicBlockInner> {
    self.inner.borrow_mut()
  }
}

pub struct BasicBlockInner {
  bb: BasicBlockRef,
  preds: Vec<BasicBlockRef>,
  insts: LinkedList<ValueAdapter>,
}

impl BasicBlockInner {
  /// Gets the predecessor list.
  pub fn preds(&self) -> &[BasicBlockRef] {
    &self.preds
  }

  /// Gets the mutable predecessor list.
  pub fn preds_mut(&mut self) -> &Vec<BasicBlockRef> {
    &mut self.preds
  }

  /// Gets the successors list.
  pub fn succs(&self) -> &[BasicBlockRef] {
    if let Some(inst) = self.insts.back().get() {
      match inst.kind() {
        ValueKind::Branch(branch) => branch.targets(),
        ValueKind::Jump(jump) => slice::from_ref(jump.target()),
        _ => &[],
      }
    } else {
      &[]
    }
  }

  /// Gets the instruction list.
  pub fn insts(&self) -> &LinkedList<ValueAdapter> {
    &self.insts
  }

  /// Adds the specific instruction to the current basic block.
  ///
  /// # Panics
  ///
  /// Panics when `inst` is not an instruction, or the instruction
  /// is already in another basic block.
  pub fn add_inst(&mut self, inst: ValueRc) {
    assert!(inst.is_inst(), "`inst` is not an instruction");
    let mut inst_inner = inst.inner_mut();
    assert!(
      inst_inner.bb().is_none(),
      "instruction is already in another basic block"
    );
    inst_inner.set_bb(Some(self.bb.clone()));
    drop(inst_inner);
    self.insts.push_back(inst);
  }

  /// Removes the specific instruction from the current basic block.
  ///
  /// # Panics
  ///
  /// Panics when the instruction is not in the current basic block.
  pub fn remove_inst(&mut self, inst: &ValueRc) {
    let mut inst_inner = inst.inner_mut();
    assert!(
      inst_inner
        .bb()
        .as_ref()
        .map_or(false, |bb| self.bb.ptr_eq(bb)),
      "instruction is not in the current basic block"
    );
    inst_inner.set_bb(None);
    unsafe {
      self.insts.cursor_mut_from_ptr(inst.as_ref()).remove();
    }
  }

  /// Replaces the specific instruction with a new instruction.
  ///
  /// # Panics
  ///
  /// Panics when the instruction is not in the current basic block, or the new
  /// value is not an instruction, or the new value is in another basic block.
  pub fn replace_inst(&mut self, inst: &ValueRc, new: ValueRc) {
    // update `inst`
    let mut inst_inner = inst.inner_mut();
    assert!(
      inst_inner
        .bb()
        .as_ref()
        .map_or(false, |bb| self.bb.ptr_eq(bb)),
      "`inst` is not in the current basic block"
    );
    inst_inner.set_bb(None);
    // update `new`
    let mut new_inner = new.inner_mut();
    assert!(new.is_inst(), "`new` is not an instruction");
    assert!(
      new_inner.bb().is_none(),
      "`new` is already in another basic block"
    );
    new_inner.set_bb(Some(self.bb.clone()));
    drop(new_inner);
    // update instruction list
    unsafe {
      let result = self
        .insts
        .cursor_mut_from_ptr(inst.as_ref())
        .replace_with(new);
      assert!(result.is_ok());
    }
  }

  /// Inserts a new instruction before the specific instruction.
  ///
  /// # Panics
  ///
  /// Panics when the instruction is not in the current basic block, or the new
  /// value is not an instruction, or the new value is in another basic block.
  pub fn insert_before(&mut self, inst: &ValueRc, new: ValueRc) {
    // check `inst`
    assert!(
      inst
        .inner()
        .bb()
        .as_ref()
        .map_or(false, |bb| self.bb.ptr_eq(bb)),
      "`inst` is not in the current basic block"
    );
    // update `new`
    let mut new_inner = new.inner_mut();
    assert!(new.is_inst(), "`new` is not an instruction");
    assert!(
      new_inner.bb().is_none(),
      "`new` is already in another basic block"
    );
    new_inner.set_bb(Some(self.bb.clone()));
    drop(new_inner);
    // update instruction list
    unsafe {
      self
        .insts
        .cursor_mut_from_ptr(inst.as_ref())
        .insert_before(new);
    }
  }

  /// Inserts a new instruction after the specific instruction.
  ///
  /// # Panics
  ///
  /// Panics when the instruction is not in the current basic block, or the new
  /// value is not an instruction, or the new value is in another basic block.
  pub fn insert_after(&mut self, inst: &ValueRc, new: ValueRc) {
    // check `inst`
    assert!(
      inst
        .inner()
        .bb()
        .as_ref()
        .map_or(false, |bb| self.bb.ptr_eq(bb)),
      "`inst` is not in the current basic block"
    );
    // update `new`
    let mut new_inner = new.inner_mut();
    assert!(new.is_inst(), "`new` is not an instruction");
    assert!(
      new_inner.bb().is_none(),
      "`new` is already in another basic block"
    );
    new_inner.set_bb(Some(self.bb.clone()));
    drop(new_inner);
    // update instruction list
    unsafe {
      self
        .insts
        .cursor_mut_from_ptr(inst.as_ref())
        .insert_after(new);
    }
  }
}

impl Drop for BasicBlockInner {
  fn drop(&mut self) {
    // handle all phi functions manually to prevent circular references
    for inst in &self.insts {
      if let ValueKind::Phi(_) = inst.kind() {
        inst.inner_mut().replace_all_uses_with(None)
      }
    }
  }
}
