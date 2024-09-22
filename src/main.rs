use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::RwLock;


fn main() -> Result<(), Error> {
    let user_id1 = UserId::new("user-1");
    let user_id2 = UserId::new("user-2");
    
    let user1 = User { id: user_id1.clone() };
    let user2 = User { id: user_id2.clone() };
    
    let mut mediator = UserMediator::default();
    
    let managed1 = user1.register(mediator.clone());
    let managed2 = user2.register(mediator.clone());
    
    let reg1= mediator.register(user_id1.clone(), managed1)?;
    let reg2 = mediator.register(user_id2.clone(), managed2)?;
    
    reg1.send_msg(&user_id2, "hi".to_string())?;
    reg2.send_msg(&user_id1, "hello".to_string())?;
    
    let user_id3 = UserId::new("user-3");
    reg1.send_msg(&user_id3, "hi".to_string())?;
    
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UserId(String);

impl UserId {
    pub fn new(id: impl Into<String>) -> UserId {
        Self(id.into())
    }
}

#[derive(Clone)]
pub struct User {
    id: UserId,
}

impl Debug for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "User id:{}", self.id.0)
    }
}

impl User {
    pub fn read_msg(&self, msg: String) {
        println!("[{}] {}", self.id.0, msg);
    }
}

impl Registered<User> {
    pub fn send_msg(&self, id: &UserId, msg: String) -> Result<(), Error> {
        self.as_mediator().consultation(self, id, msg)?;
        Ok(())
    }
}

impl Colleague for User {
    type Identifier = UserId;
    type Mediator = UserMediator;
    
    fn id(&self) -> &Self::Identifier {
        &self.id
    }
    
    fn register(self, bus: Self::Mediator) -> Managed<Self> {
        Managed::new(self, bus)
    }
}

pub struct Managed<T: Colleague> {
    inner: T,
    mediator: T::Mediator
}

impl<T: Colleague> Managed<T> {
    fn new(t: T, bus: T::Mediator) -> Self {
        Self { inner: t, mediator: bus }
    }
}

impl<T: Colleague> Deref for Managed<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct Registered<T: Colleague>(Rc<Managed<T>>);

impl<T: Colleague> Registered<T> {
    pub fn as_mediator(&self) -> &T::Mediator {
        &self.0.mediator
    }
}

impl<T: Colleague> Clone for Registered<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: Colleague> Deref for Registered<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        &self.0.inner
    }
}

pub trait Colleague: Sized {
    type Identifier;
    type Mediator: Mediator<Self>;
    fn id(&self) -> &Self::Identifier;
    fn register(self, mediator: Self::Mediator) -> Managed<Self>;
}

pub trait Mediator<T: Colleague> {
    fn register(&mut self, id: T::Identifier, registered: Managed<T>) -> Result<Registered<T>, Error>;
    fn consultation(&self, user: &T, to: &T::Identifier, msg: String) -> Result<(), Error>;
}

#[derive(Default)]
pub struct UserMediator {
    users: Rc<RwLock<HashMap<UserId, Registered<User>>>>
}

impl Clone for UserMediator {
    fn clone(&self) -> Self {
        Self { users: Rc::clone(&self.users) }
    }
}

impl Mediator<User> for UserMediator {
    fn register(&mut self, id: UserId, registered: Managed<User>) -> Result<Registered<User>, Error> {
        let reg = Registered(Rc::new(registered));
        self.users.write().map_err(|_| Error::LockPoison)?
            .insert(id, reg.clone());
        Ok(reg)
    }

    fn consultation(&self, from: &User, to: &UserId, msg: String) -> Result<(), Error> {
        match self.users.read()
            .map_err(|_| Error::LockPoison)?
            .iter()
            .find(|(id, _)| id.eq(&to)) 
        {
            Some((_, user)) => {
                println!("[Mediator] from:{:?} -> to:{:?}: {}", from.id, to, msg);
                user.read_msg(msg);
            }
            None => {
                println!("[Mediator] msg:{} from {:?} has drifted over to deadletter.", msg, from.id);
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot lock")]
    LockPoison
}
