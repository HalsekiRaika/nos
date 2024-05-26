#![allow(unused)]

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use uuid::Uuid;
use diazene::actor::{Actor, ActorRef, Context, Handler, Message, RegularBehavior};
use diazene::system::ActorSystem;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct PersonId(Uuid);

impl Display for PersonId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Person({})", self.0)
    }
}

impl Default for PersonId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    id: Uuid,
    title: String,
    rental: HashSet<PersonId>
}

#[derive(Debug, Clone)]
pub enum BookCommand {
    Rental { id: PersonId },
    Return { id: PersonId },
    Archive,
}

#[derive(Debug, Clone)]
pub enum Error {
    AlreadyExist { reason: String },
    NotFound { reason: String }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::AlreadyExist { reason } => write!(f, "{}", reason),
            Error::NotFound { reason } => write!(f, "{}", reason),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub enum BookEvent {
    Rental { id: PersonId },
    Returned { id: PersonId },
    Archived,
}

impl Message for BookCommand {}

impl Actor for Book {}

impl Handler<BookCommand> for Book {
    type Accept = BookEvent;
    type Rejection = Error;

    #[tracing::instrument(skip_all, name = "book-cmd-handler")]
    async fn handle(&mut self, msg: BookCommand, ctx: &mut Context) -> Result<Self::Accept, Self::Rejection> {
        match msg {
            BookCommand::Rental { id } => {
                if !self.rental.insert(id) {
                    return Err(Error::AlreadyExist {
                        reason: format!("The book is already on loan by {}.", id)
                    })
                }
                tracing::debug!("rental={}", id);
                Ok(BookEvent::Rental { id })
            }
            BookCommand::Return { id } => {
                if !self.rental.remove(&id) {
                    return Err(Error::NotFound {
                        reason: format!("This book is not on loan from {}.", id)
                    })
                }
                tracing::debug!("return={}", id);
                Ok(BookEvent::Returned { id })
            },
            BookCommand::Archive => {
                tracing::info!("book archived. (self shutdown)");
                ctx.shutdown();
                Ok(BookEvent::Archived)
            },
        }
    }
}


fn create_book() -> (Uuid, Book) {
    let id = Uuid::new_v4();
    let book = Book {
        id,
        title: "Charlie and the Chocolate Factory".to_string(),
        rental: Default::default(),
    };

    (id, book)
}

async fn find_or(system: &ActorSystem) -> anyhow::Result<()> {
    let (id, book) = create_book();
    
    let _ = system.spawn(id, book).await?;
    
    let refs: ActorRef<Book> = system.find_or(id, |_id| async {
        unreachable!()
    }).await?;
    
    refs.tell(BookCommand::Archive).await??;
    
    let id = Uuid::new_v4();

    let refs = system.find_or(id, |id| async move {
        Book {
            id,
            title: "The Book of Rust :ferris:".to_string(),
            rental: Default::default(),
        }
    }).await?;
    
    let ev1 = refs.ask(BookCommand::Rental { id: PersonId::default() }).await??;
    let ev2 = refs.ask(BookCommand::Archive).await??;
    
    tracing::debug!("{:?}", ev1);
    tracing::debug!("{:?}", ev2);
    
    Ok(())
}

#[tokio::test]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer()
                  .with_filter(tracing_subscriber::EnvFilter::new("test=trace,diazene=trace"))
                  .with_filter(tracing_subscriber::filter::LevelFilter::TRACE),
        )
        .init();
    
    let system = ActorSystem::new();
    find_or(&system).await?;
    Ok(())
}