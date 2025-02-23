use crate::{
    algebra::{AddAssignByRef, AddByRef, NegByRef},
    time::AntichainRef,
    trace::{
        layers::{
            file::column_layer::{
                FileColumnLayer, FileColumnLayerBuilder, FileColumnLayerConsumer,
                FileColumnLayerCursor, FileColumnLayerValues,
            },
            Builder as TrieBuilder, Cursor as TrieCursor, MergeBuilder, Trie, TupleBuilder,
        },
        ord::merge_batcher::MergeBatcher,
        Batch, BatchReader, Builder, Consumer, Cursor, Filter, Merger, ValueConsumer,
    },
    DBData, DBWeight, NumEntries,
};
use rand::Rng;
use rkyv::{Archive, Deserialize, Serialize};
use size_of::SizeOf;
use std::{
    cmp::max,
    fmt::{self, Debug, Display},
    ops::{Add, AddAssign, Neg},
    rc::Rc,
};

/// A batch of weighted tuples without values or times.
///
/// Each tuple in `FileZSet<K, R>` has key type `K`, value type `()`, weight
/// type `R`, and time type `()`.
#[derive(Debug, Clone, SizeOf, Archive, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq))]
pub struct FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    #[doc(hidden)]
    pub layer: FileColumnLayer<K, R>,
}

impl<K, R> FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.layer.len() as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.layer.is_empty()
    }

    #[inline]
    pub fn retain<F>(&mut self, retain: F)
    where
        F: Fn(&K, &R) -> bool,
    {
        let mut cursor = self.cursor();
        let mut builder = FileColumnLayerBuilder::new();
        while cursor.key_valid() {
            while cursor.val_valid() {
                let weight = cursor.weight();
                let key = cursor.key();
                if retain(key, &weight) {
                    builder.push_tuple((key.clone(), weight));
                }
                cursor.step_val();
            }
            cursor.step_key();
        }
    }
}

// This is `#[cfg(test)]` only because it would be surprisingly expensive in
// production.
impl<Other, K, R> PartialEq<Other> for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
    Other: BatchReader<Key = K, Val = (), R = R, Time = ()>,
{
    fn eq(&self, other: &Other) -> bool {
        let mut c1 = self.cursor();
        let mut c2 = other.cursor();
        while c1.key_valid() && c2.key_valid() {
            if c1.key() != c2.key() || c1.weight() != c2.weight() {
                return false;
            }
            c1.step_key();
            c2.step_key();
        }
        !c1.key_valid() && !c2.key_valid()
    }
}

impl<K, R> Display for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "layer:\n{}",
            textwrap::indent(&self.layer.to_string(), "    ")
        )
    }
}

impl<K, R> From<FileColumnLayer<K, R>> for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    fn from(layer: FileColumnLayer<K, R>) -> Self {
        Self { layer }
    }
}

impl<K, R> From<FileColumnLayer<K, R>> for Rc<FileZSet<K, R>>
where
    K: DBData,
    R: DBWeight,
{
    fn from(layer: FileColumnLayer<K, R>) -> Self {
        Rc::new(From::from(layer))
    }
}

impl<K, R> NumEntries for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    const CONST_NUM_ENTRIES: Option<usize> = <FileColumnLayer<K, R>>::CONST_NUM_ENTRIES;

    fn num_entries_shallow(&self) -> usize {
        self.layer.num_entries_shallow()
    }

    fn num_entries_deep(&self) -> usize {
        self.layer.num_entries_deep()
    }
}

impl<K, R> Default for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    fn default() -> Self {
        Self {
            layer: FileColumnLayer::empty(),
        }
    }
}

impl<K, R> NegByRef for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight + NegByRef,
{
    fn neg_by_ref(&self) -> Self {
        Self {
            layer: self.layer.neg_by_ref(),
        }
    }
}

impl<K, R> Neg for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight + Neg<Output = R>,
{
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            layer: self.layer.neg(),
        }
    }
}

// TODO: by-value merge
impl<K, R> Add<Self> for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            layer: self.layer.add(rhs.layer),
        }
    }
}

impl<K, R> AddAssign<Self> for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    fn add_assign(&mut self, rhs: Self) {
        self.layer.add_assign(rhs.layer);
    }
}

impl<K, R> AddAssignByRef for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    fn add_assign_by_ref(&mut self, rhs: &Self) {
        self.layer.add_assign_by_ref(&rhs.layer);
    }
}

impl<K, R> AddByRef for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    fn add_by_ref(&self, rhs: &Self) -> Self {
        Self {
            layer: self.layer.add_by_ref(&rhs.layer),
        }
    }
}

impl<K, R> BatchReader for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    type Key = K;
    type Val = ();
    type Time = ();
    type R = R;
    type Cursor<'s> = FileZSetCursor<'s, K, R>;
    type Consumer = FileZSetConsumer<K, R>;

    #[inline]
    fn cursor(&self) -> Self::Cursor<'_> {
        FileZSetCursor {
            valid: true,
            cursor: self.layer.cursor(),
        }
    }

    #[inline]
    fn consumer(self) -> Self::Consumer {
        FileZSetConsumer {
            consumer: FileColumnLayerConsumer::from(self.layer),
        }
    }

    #[inline]
    fn key_count(&self) -> usize {
        Trie::keys(&self.layer)
    }

    #[inline]
    fn len(&self) -> usize {
        self.layer.tuples()
    }

    #[inline]
    fn lower(&self) -> AntichainRef<'_, ()> {
        AntichainRef::new(&[()])
    }

    #[inline]
    fn upper(&self) -> AntichainRef<'_, ()> {
        AntichainRef::empty()
    }

    fn truncate_keys_below(&mut self, lower_bound: &Self::Key) {
        self.layer.truncate_keys_below(lower_bound);
    }

    fn sample_keys<RG>(&self, rng: &mut RG, sample_size: usize, sample: &mut Vec<Self::Key>)
    where
        RG: Rng,
    {
        self.layer.sample_keys(rng, sample_size, sample);
    }
}

impl<K, R> Batch for FileZSet<K, R>
where
    K: DBData,
    R: DBWeight,
{
    type Item = K;
    type Batcher = MergeBatcher<K, (), R, Self>;
    type Builder = FileZSetBuilder<K, R>;
    type Merger = FileZSetMerger<K, R>;

    fn item_from(key: K, _val: ()) -> Self::Item {
        key
    }

    fn from_keys(time: Self::Time, keys: Vec<(Self::Key, Self::R)>) -> Self {
        Self::from_tuples(time, keys)
    }

    fn begin_merge(&self, other: &Self) -> Self::Merger {
        FileZSetMerger::new_merger(self, other)
    }

    fn recede_to(&mut self, _frontier: &()) {}

    fn empty(_time: Self::Time) -> Self {
        Self {
            layer: FileColumnLayer::empty(),
        }
    }
}

/// State for an in-progress merge.
#[derive(SizeOf)]
pub struct FileZSetMerger<K, R>
where
    K: DBData,
    R: DBWeight,
{
    // result that we are currently assembling.
    result: <FileColumnLayer<K, R> as Trie>::MergeBuilder,
}

impl<K, R> Merger<K, (), (), R, FileZSet<K, R>> for FileZSetMerger<K, R>
where
    Self: SizeOf,
    K: DBData,
    R: DBWeight,
{
    fn new_merger(batch1: &FileZSet<K, R>, batch2: &FileZSet<K, R>) -> Self {
        Self {
            result: <<FileColumnLayer<K, R> as Trie>::MergeBuilder as MergeBuilder>::with_capacity(
                &batch1.layer,
                &batch2.layer,
            ),
        }
    }

    fn done(self) -> FileZSet<K, R> {
        FileZSet {
            layer: self.result.done(),
        }
    }

    fn work(
        &mut self,
        source1: &FileZSet<K, R>,
        source2: &FileZSet<K, R>,
        key_filter: &Option<Filter<K>>,
        _value_filter: &Option<Filter<()>>,
        fuel: &mut isize,
    ) {
        let initial_size = self.result.keys();

        if let Some(key_filter) = key_filter {
            self.result.push_merge_retain_keys(
                source1.layer.cursor(),
                source2.layer.cursor(),
                key_filter,
            );
        } else {
            self.result
                .push_merge(source1.layer.cursor(), source2.layer.cursor());
        }
        let effort = self.result.keys() - initial_size;
        *fuel -= effort as isize;

        *fuel = max(*fuel, 1);
    }
}

/// A cursor for navigating a single layer.
#[derive(Debug, SizeOf, Clone)]
pub struct FileZSetCursor<'s, K, R>
where
    K: DBData,
    R: DBWeight,
{
    valid: bool,
    cursor: FileColumnLayerCursor<'s, K, R>,
}

impl<'s, K, R> Cursor<K, (), (), R> for FileZSetCursor<'s, K, R>
where
    K: DBData,
    R: DBWeight,
{
    fn key(&self) -> &K {
        self.cursor.current_key()
    }

    fn val(&self) -> &() {
        &()
    }

    fn fold_times<F, U>(&mut self, init: U, mut fold: F) -> U
    where
        F: FnMut(U, &(), &R) -> U,
    {
        if self.cursor.valid() {
            fold(init, &(), self.cursor.current_diff())
        } else {
            init
        }
    }

    fn fold_times_through<F, U>(&mut self, _upper: &(), init: U, fold: F) -> U
    where
        F: FnMut(U, &(), &R) -> U,
    {
        self.fold_times(init, fold)
    }

    fn weight(&mut self) -> R {
        debug_assert!(&self.cursor.valid());
        self.cursor.current_diff().clone()
    }

    fn key_valid(&self) -> bool {
        self.cursor.valid()
    }

    fn val_valid(&self) -> bool {
        self.valid
    }

    fn step_key(&mut self) {
        self.cursor.step();
        self.valid = true;
    }

    fn step_key_reverse(&mut self) {
        self.cursor.step_reverse();
        self.valid = true;
    }

    fn seek_key(&mut self, key: &K) {
        self.cursor.seek(key);
        self.valid = true;
    }

    fn seek_key_with<P>(&mut self, predicate: P)
    where
        P: Fn(&K) -> bool + Clone,
    {
        self.cursor.seek_with(|k| !predicate(k));
        self.valid = true;
    }

    fn seek_key_with_reverse<P>(&mut self, predicate: P)
    where
        P: Fn(&K) -> bool + Clone,
    {
        self.cursor.seek_with_reverse(|k| !predicate(k));
        self.valid = true;
    }

    fn seek_key_reverse(&mut self, key: &K) {
        self.cursor.seek_reverse(key);
        self.valid = true;
    }

    fn step_val(&mut self) {
        self.valid = false;
    }

    fn seek_val(&mut self, _val: &()) {}

    fn seek_val_with<P>(&mut self, predicate: P)
    where
        P: Fn(&()) -> bool + Clone,
    {
        if !predicate(&()) {
            self.valid = false;
        }
    }

    fn rewind_keys(&mut self) {
        self.cursor.rewind();
        self.valid = true;
    }

    fn fast_forward_keys(&mut self) {
        self.cursor.fast_forward();
        self.valid = true;
    }

    fn rewind_vals(&mut self) {
        self.valid = true;
    }

    fn step_val_reverse(&mut self) {
        self.valid = false;
    }

    fn seek_val_reverse(&mut self, _val: &()) {}

    fn seek_val_with_reverse<P>(&mut self, predicate: P)
    where
        P: Fn(&()) -> bool + Clone,
    {
        if !predicate(&()) {
            self.valid = false;
        }
    }

    fn fast_forward_vals(&mut self) {
        self.valid = true;
    }
}

/// A builder for creating layers from unsorted update tuples.
#[derive(SizeOf)]
pub struct FileZSetBuilder<K, R>
where
    K: DBData,
    R: DBWeight,
{
    builder: FileColumnLayerBuilder<K, R>,
}

impl<K, R> Builder<K, (), R, FileZSet<K, R>> for FileZSetBuilder<K, R>
where
    Self: SizeOf,
    K: DBData,
    R: DBWeight,
{
    #[inline]
    fn new_builder(_time: ()) -> Self {
        Self {
            builder: FileColumnLayerBuilder::new(),
        }
    }

    #[inline]
    fn with_capacity(_time: (), capacity: usize) -> Self {
        Self {
            builder: <FileColumnLayerBuilder<K, R> as TupleBuilder>::with_capacity(capacity),
        }
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.builder.reserve(additional);
    }

    #[inline]
    fn push(&mut self, (key, diff): (K, R)) {
        self.builder.push_tuple((key, diff));
    }

    #[inline(never)]
    fn done(self) -> FileZSet<K, R> {
        FileZSet {
            layer: self.builder.done(),
        }
    }
}

#[derive(Debug, SizeOf)]
pub struct FileZSetConsumer<K, R>
where
    K: DBData,
    R: DBWeight,
{
    consumer: FileColumnLayerConsumer<K, R>,
}

impl<K, R> Consumer<K, (), R, ()> for FileZSetConsumer<K, R>
where
    K: DBData,
    R: DBWeight,
{
    type ValueConsumer<'a> = FileZSetValueConsumer<R>
    where
        Self: 'a;

    fn key_valid(&self) -> bool {
        self.consumer.key_valid()
    }

    fn peek_key(&self) -> &K {
        self.consumer.peek_key()
    }

    fn next_key(&mut self) -> (K, Self::ValueConsumer<'_>) {
        let (key, values) = self.consumer.next_key();
        (key, FileZSetValueConsumer { values })
    }

    fn seek_key(&mut self, key: &K)
    where
        K: Ord,
    {
        self.consumer.seek_key(key);
    }
}

#[derive(Debug)]
pub struct FileZSetValueConsumer<R>
where
    R: DBWeight,
{
    values: FileColumnLayerValues<R>,
}

impl<'a, R> ValueConsumer<'a, (), R, ()> for FileZSetValueConsumer<R>
where
    R: DBWeight,
{
    fn value_valid(&self) -> bool {
        self.values.value_valid()
    }

    fn next_value(&mut self) -> ((), R, ()) {
        self.values.next_value()
    }

    fn remaining_values(&self) -> usize {
        self.values.remaining_values()
    }
}
