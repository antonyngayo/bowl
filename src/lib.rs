use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap},
};

#[derive(Debug, PartialEq, Default, Clone, Copy, Eq, PartialOrd, Ord, Hash)]
pub enum ConversionState {
    #[default]
    Runnable,
    Running,
    Finished,
    Failed,
}

pub trait MediaTrait {
    fn get_name(&self) -> &str; // this is the name of the file
    fn get_uuid(&self) -> &str;
    fn get_state(&self) -> &ConversionState;
    fn get_organization(&self) -> &str;
    fn set_state(&mut self, state: ConversionState);
}

#[allow(unused)]
#[derive(Debug, Default)]
pub struct Bowl {
    contents: BTreeMap<
        TypeId,
        HashMap<
            String, // Organization name
            HashMap<
                String, // UUID for the file
                std::boxed::Box<
                    (dyn std::any::Any + std::marker::Send + std::marker::Sync + 'static),
                >,
            >,
        >,
    >,
}
/// Implementing an extension trait for Bowl which is generic over any Iterator type that implements
/// the MediaTrait trait
impl<U: MediaTrait + std::fmt::Debug + Send + Sync + 'static + Clone> Extend<U> for Bowl {
    fn extend<T: IntoIterator<Item = U>>(&mut self, iter: T) {
        for file in iter {
            let org = file.get_organization();
            self.contents
                .entry(TypeId::of::<U>())
                .or_default()
                .entry(org.into())
                .or_default()
                .insert(file.get_uuid().to_string(), Box::new(file));
        }
    }
}

#[allow(unused)]
impl Bowl {
    pub fn new() -> Self {
        Self {
            contents: BTreeMap::new(),
        }
    }

    pub fn add<T: Any + MediaTrait + std::fmt::Debug + Send + Sync + 'static>(
        &mut self,
        org: &str,
        value: T,
    ) {
        // insert value based on type and uuid
        self.contents
            .entry(TypeId::of::<T>())
            .or_default()
            .entry(org.into())
            .or_default()
            .insert(value.get_uuid().to_string(), Box::new(value));
    }

    // getting one file based on type and uuid
    pub fn get<T: Any + std::fmt::Debug + MediaTrait + Send + Sync>(
        &self,
        org: &str,
        uuid: &str,
    ) -> Option<&T> {
        self.contents.get(&TypeId::of::<T>()).and_then(|b| {
            b.get(org)
                .and_then(|x| x.get(uuid).unwrap().downcast_ref::<T>().to_owned())
        })
    }

    pub fn update_state<T: Any + std::fmt::Debug + MediaTrait + Send + Sync>(
        &mut self,
        uuid: &str,
        org: &str,
        state: ConversionState,
    ) {
        self.contents
            .get_mut(&TypeId::of::<T>())
            .and_then(|org_hash| {
                org_hash.get_mut(org).map(|target_org| {
                    target_org
                        .get_mut(uuid)
                        .and_then(|file| file.downcast_mut::<T>().map(|x| x.set_state(state)))
                })
            });
    }

    // deleting a file based on type and uuid
    pub fn delete<T: Any + std::fmt::Debug + MediaTrait + Send + Sync>(
        &mut self,
        org: &str,
        uuid: &str,
    ) {
        self.contents
            .get_mut(&TypeId::of::<T>())
            .and_then(|target| target.get_mut(org).and_then(|mark| mark.remove(uuid)));
    }

    // get_all
    pub fn get_all<T: Any + std::fmt::Debug + MediaTrait + Send + Sync>(
        &self,
        org: &str,
    ) -> Vec<&T> {
        self.contents
            .get(&TypeId::of::<T>())
            .and_then(|orgn| orgn.get(org))
            .map(|tg| {
                tg.iter()
                    .map(|(_, v)| v.downcast_ref::<T>().unwrap())
                    .collect()
            })
            .unwrap_or(vec![])
    }

    pub fn filter_by_org_and_state<T: Any + std::fmt::Debug + MediaTrait + Send + Sync>(
        &self,
        org: &str,
        state: ConversionState,
    ) -> Vec<&T> {
        self.contents
            .get(&TypeId::of::<T>())
            .and_then(|org_hash| org_hash.get(org))
            .map(|target_org| {
                target_org
                    .iter()
                    .filter(|(k, v)| v.downcast_ref::<T>().unwrap().get_state() == &state)
                    .map(|(k, v)| v.downcast_ref::<T>().unwrap())
                    .collect()
            })
            .unwrap_or(vec![])
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    #[derive(Debug, PartialEq, Default, Clone, Eq, PartialOrd, Ord, Hash)]
    struct MediaFile<'a> {
        name: Cow<'a, str>,
        uuid: Cow<'a, str>,
        state: ConversionState,
        organization: Cow<'a, str>,
    }

    impl MediaTrait for MediaFile<'_> {
        fn get_name(&self) -> &str {
            &self.name
        }
        fn get_uuid(&self) -> &str {
            &self.uuid
        }
        fn get_state(&self) -> &ConversionState {
            &self.state
        }
        fn get_organization(&self) -> &str {
            &self.organization
        }
        fn set_state(&mut self, state: ConversionState) {
            self.state = state;
        }
    }

    #[test]
    fn test_add() {
        let mut bowl = Bowl::new();
        let file = MediaFile {
            name: "test.mp4".into(),
            uuid: "1234".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };

        bowl.add(file.get_organization(), file.clone());
        assert_eq!(bowl.get_all::<MediaFile>("test").len(), 1);
    }

    #[test]
    fn test_get() {
        let mut bowl = Bowl::new();
        let file = MediaFile {
            name: "test.mp4".into(),
            uuid: "1234".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };
        bowl.add(file.get_organization(), file.clone());
        assert_eq!(
            bowl.get::<MediaFile>("test", "1234").unwrap().get_uuid(),
            "1234"
        );
    }

    #[test]
    fn test_get_by_org_and_state() {
        let mut bowl = Bowl::new();
        let file = MediaFile {
            name: "test.mp4".into(),
            uuid: "1234".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };
        bowl.add(file.get_organization(), file.clone());
        assert_eq!(
            bowl.filter_by_org_and_state::<MediaFile>("test", ConversionState::Runnable)
                .len(),
            1
        );
    }

    #[test]
    fn test_delete() {
        let mut bowl = Bowl::new();
        let file = MediaFile {
            name: "test.mp4".into(),
            uuid: "1234".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };
        bowl.add(file.get_organization(), file.clone());
        assert_eq!(bowl.get_all::<MediaFile>("test").len(), 1);
        bowl.delete::<MediaFile>("test", "1234");
        assert_eq!(bowl.get_all::<MediaFile>("test").len(), 0);
    }

    // write a fuzzer for this with random data and see if it works
    #[test]
    fn test_fuzzer() {
        let mut bowl = Bowl::new();
        let file = MediaFile {
            name: "test.mp4".into(),
            uuid: "12341".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };
        // add more files and use extend to add to the bowl and see if it works

        let file2 = MediaFile {
            name: "test2.mp4".into(),
            uuid: "12342".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };

        let file3 = MediaFile {
            name: "test3.mp4".into(),
            uuid: "12343".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };

        let file4 = MediaFile {
            name: "test4.mp4".into(),
            uuid: "12344".into(),
            state: ConversionState::Runnable,
            organization: "test".into(),
        };

        let files = vec![file, file2, file3, file4];
        bowl.extend(files);
        assert_eq!(bowl.get_all::<MediaFile>("test").len(), 4);
    }
}
