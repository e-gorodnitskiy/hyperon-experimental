use hyperon::Atom;
use hyperon::metta::text::*;
use hyperon::metta::interpreter;
use hyperon::metta::interpreter::InterpretedAtom;
use hyperon::common::plan::StepResult;
use hyperon::metta::runner::Metta;

use crate::util::*;
use crate::atom::*;
use crate::space::*;

use std::os::raw::*;
use regex::Regex;
use std::path::PathBuf;

// Tokenizer

pub type tokenizer_t = SharedApi<Tokenizer>;

#[no_mangle]
pub extern "C" fn tokenizer_new() -> *mut tokenizer_t {
    tokenizer_t::new(Tokenizer::new())
}

#[no_mangle]
pub extern "C" fn tokenizer_free(tokenizer: *mut tokenizer_t) {
    tokenizer_t::drop(tokenizer)
}

type atom_constr_t = extern "C" fn(*const c_char, *mut c_void) -> *mut atom_t;

#[repr(C)]
pub struct droppable_t {
    ptr: *mut c_void,
    free: Option<extern "C" fn(ptr: *mut c_void)>,
}

impl Drop for droppable_t {
    fn drop(&mut self) {
        let free = (*self).free;
        if let Some(free) = free {
            free(self.ptr);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tokenizer_register_token(tokenizer: *mut tokenizer_t,
    regex: *const c_char, constr: atom_constr_t, context: droppable_t) {
    let regex = Regex::new(cstr_as_str(regex)).unwrap();
    (*tokenizer).borrow_mut().register_token(regex, move |token| {
        let catom = Box::from_raw(constr(str_as_cstr(token).as_ptr(), context.ptr));
        catom.atom
    });
}

#[no_mangle]
pub extern "C" fn tokenizer_clone(tokenizer: *const tokenizer_t) -> *mut tokenizer_t {
    let copy = unsafe { (*tokenizer).borrow().clone() };
    tokenizer_t::new(copy)
}

// SExprParser

pub type sexpr_parser_t<'a> = SharedApi<SExprParser<'a>>;

#[no_mangle]
pub extern "C" fn sexpr_parser_new<'a>(text: *const c_char) -> *mut sexpr_parser_t<'a> {
    sexpr_parser_t::new(SExprParser::new(cstr_as_str(text)))
}

#[no_mangle]
pub extern "C" fn sexpr_parser_free(parser: *mut sexpr_parser_t) {
    sexpr_parser_t::drop(parser)
}

#[no_mangle]
pub unsafe extern "C" fn sexpr_parser_parse(parser: *mut sexpr_parser_t,
        tokenizer: *const tokenizer_t) -> *mut atom_t {
    (*parser).borrow_mut().parse(&(*tokenizer).borrow())
        .map_or(std::ptr::null_mut(), |atom| { atom_to_ptr(atom) })
}

#[no_mangle] pub extern "C" fn ATOM_TYPE_UNDEFINED() -> *mut atom_t { atom_to_ptr(hyperon::metta::ATOM_TYPE_UNDEFINED) }
#[no_mangle] pub extern "C" fn ATOM_TYPE_TYPE() -> *mut atom_t { atom_to_ptr(hyperon::metta::ATOM_TYPE_TYPE) }
#[no_mangle] pub extern "C" fn ATOM_TYPE_ATOM() -> *mut atom_t { atom_to_ptr(hyperon::metta::ATOM_TYPE_ATOM) }
#[no_mangle] pub extern "C" fn ATOM_TYPE_SYMBOL() -> *mut atom_t { atom_to_ptr(hyperon::metta::ATOM_TYPE_SYMBOL) }
#[no_mangle] pub extern "C" fn ATOM_TYPE_VARIABLE() -> *mut atom_t { atom_to_ptr(hyperon::metta::ATOM_TYPE_VARIABLE) }
#[no_mangle] pub extern "C" fn ATOM_TYPE_EXPRESSION() -> *mut atom_t { atom_to_ptr(hyperon::metta::ATOM_TYPE_EXPRESSION) }
#[no_mangle] pub extern "C" fn ATOM_TYPE_GROUNDED() -> *mut atom_t { atom_to_ptr(hyperon::metta::ATOM_TYPE_GROUNDED) }

#[no_mangle]
pub unsafe extern "C" fn check_type(space: *const grounding_space_t, atom: *const atom_t, typ: *const atom_t) -> bool {
    hyperon::metta::types::check_type(&(*space).borrow(), &(*atom).atom, &(*typ).atom)
}

#[no_mangle]
pub unsafe extern "C" fn validate_atom(space: *const grounding_space_t, atom: *const atom_t) -> bool {
    hyperon::metta::types::validate_atom(&(*space).borrow(), &(*atom).atom)
}

#[no_mangle]
pub extern "C" fn get_atom_types(space: *const grounding_space_t, atom: *const atom_t,
        callback: c_atoms_callback_t, context: *mut c_void) {
    let space = unsafe{ &(*space).borrow() };
    let atom = unsafe{ &(*atom).atom };
    let types = hyperon::metta::types::get_atom_types(space, atom);
    return_atoms(&types, callback, context);
}

// MeTTa interpreter API

pub struct step_result_t<'a> {
    result: StepResult<'a, Vec<InterpretedAtom>, (Atom, Atom)>,
}

#[no_mangle]
pub extern "C" fn interpret_init<'a>(space: *mut grounding_space_t, expr: *const atom_t) -> *mut step_result_t<'a> {
    let space = unsafe{ &(*space) };
    let expr = unsafe{ &(*expr) };
    let step = interpreter::interpret_init(space.shared(), &expr.atom);
    Box::into_raw(Box::new(step_result_t{ result: step }))
}

#[no_mangle]
pub extern "C" fn interpret_step(step: *mut step_result_t) -> *mut step_result_t {
    let step = unsafe { Box::from_raw(step) };
    let next = interpreter::interpret_step(step.result);
    Box::into_raw(Box::new(step_result_t{ result: next }))
}

#[no_mangle]
pub extern "C" fn step_has_next(step: *const step_result_t) -> bool {
    unsafe{ (*step).result.has_next() }
}

#[no_mangle]
pub extern "C" fn step_get_result(step: *mut step_result_t,
        callback: c_atoms_callback_t, context: *mut c_void) {
    let step = unsafe{ Box::from_raw(step) };
    match step.result {
        StepResult::Return(mut res) => {
            let res = res.drain(0..).map(|res| res.into_tuple().0).collect();
            return_atoms(&res, callback, context);
        },
        StepResult::Error(_) => return_atoms(&vec![], callback, context),
        _ => panic!("Not expected step result: {:?}", step.result),
    }
}

#[no_mangle]
pub extern "C" fn step_to_str(step: *const step_result_t, callback: c_str_callback_t, context: *mut c_void) {
    let result = unsafe{ &(*step).result };
    callback(str_as_cstr(format!("{:?}", result).as_str()).as_ptr(), context);
}

pub type metta_t = SharedApi<Metta>;

#[no_mangle]
pub extern "C" fn metta_new(space: *mut grounding_space_t, tokenizer: *mut tokenizer_t, cwd: *const c_char) -> *mut metta_t {
    let space = unsafe{ &mut *space }.shared();
    let tokenizer = unsafe{ &mut *tokenizer }.shared();
    metta_t::new(Metta::from_space_cwd(space, tokenizer, PathBuf::from(cstr_as_str(cwd))))
}

#[no_mangle]
pub extern "C" fn metta_free(metta: *mut metta_t) {
    metta_t::drop(metta);
}

#[no_mangle]
pub extern "C" fn metta_space(metta: *mut metta_t) -> *mut grounding_space_t {
    let space = unsafe{ &*metta }.borrow().space();
    grounding_space_t::from_shared(space)
}

#[no_mangle]
pub extern "C" fn metta_tokenizer(metta: *mut metta_t) -> *mut tokenizer_t {
    let tokenizer = unsafe{ &*metta }.borrow().tokenizer();
    tokenizer_t::from_shared(tokenizer)
}

#[no_mangle]
pub extern "C" fn metta_run(metta: *mut metta_t, parser: *mut sexpr_parser_t,
        output: c_atoms_callback_t, out_context: *mut c_void) {
    let metta = unsafe{ &*metta }.borrow();
    let mut parser = unsafe{ &mut *parser }.borrow_mut();
    let results = metta.run(&mut parser);
    // TODO: return erorrs properly after step_get_result() is changed to return errors.
    for result in results.expect("Returning errors from C API is not implemented yet") {
        return_atoms(&result, output, out_context);
    }
}
