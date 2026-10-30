use crate::analyzer::{
    static_env::VarId,
    types::{
        AbstractType,
        bounds::{Satisfaction, TypeBound, TypeSatisfaction, error::UnsatisfiedBoundError},
    },
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum GiveTypeError<'ent, 'src> {
    Existing(&'ent AbstractType<'src>),
    Incompatible(Box<UnsatisfiedBoundError<'src>>),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Default)]
pub struct Entry<'src>(Option<AbstractType<'src>>);

impl<'src> Entry<'src> {
    pub fn give_type(&mut self, ty: AbstractType<'src>) -> Result<(), GiveTypeError<'_, 'src>> {
        if let Some(existing) = &mut self.0 {
            if let AbstractType::Opaque(bound) = existing {
                match ty.satisfaction_of(bound) {
                    Satisfaction::Passing(_) => {
                        self.0 = Some(ty);
                        Ok(())
                    }

                    Satisfaction::Failing(e) => {
                        Err(GiveTypeError::Incompatible(Box::new(e.into())))
                    }

                    Satisfaction::Unclear(_) => todo!("restrained type"),
                }
            } else {
                Err(GiveTypeError::Existing(existing))
            }
        } else {
            self.0 = Some(ty);
            Ok(())
        }
    }

    pub fn add_bound(
        &mut self,
        mut bound: TypeBound<'src>,
    ) -> Result<(), Box<UnsatisfiedBoundError<'src>>> {
        if let Some(existing) = &mut self.0 {
            match existing.satisfaction_of(&bound) {
                Satisfaction::Passing(_) => Ok(()),
                Satisfaction::Failing(e) => Err(Box::new(e.into())),

                Satisfaction::Unclear(_) => {
                    if let AbstractType::Opaque(existing) = existing {
                        existing.extend_one(bound);
                    } else {
                        bound.extend_one(TypeBound::Exactly(std::mem::take(existing)));
                        *existing = AbstractType::Opaque(Box::new(bound));
                    }
                    Ok(())
                }
            }
        } else {
            self.0 = Some(AbstractType::Opaque(Box::new(bound)));
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct MorphEnvironment<'src> {
    items: HashMap<VarId, Entry<'src>>,
}

impl<'src> MorphEnvironment<'src> {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn get(&self, id: VarId) -> Option<&Entry<'src>> {
        self.items.get(&id)
    }

    pub fn get_mut(&mut self, id: VarId) -> Option<&mut Entry<'src>> {
        self.items.get_mut(&id)
    }
}
