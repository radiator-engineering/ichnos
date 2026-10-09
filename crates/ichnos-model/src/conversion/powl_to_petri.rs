//! POWL to Petri net, ported from pm4py's
//! `objects/conversion/powl/variants/to_petri_net.py`.

use super::tree_to_petri::{Builder, Entity};
use crate::petri::{AcceptingPetriNet, PlaceId};
use crate::powl::{Powl, PowlError};

impl Builder {
    /// pm4py's `recursively_add_tree` for POWL. Returns the final place of
    /// the submodel.
    fn add_powl(
        &mut self,
        powl: &Powl,
        initial: Entity,
        fin: Option<Entity>,
    ) -> Result<PlaceId, PowlError> {
        let (initial_place, final_place) = self.attach(initial, fin);
        match powl {
            Powl::Silent | Powl::Activity(_) | Powl::Frequent(_) => {
                let t = match powl.label() {
                    Some(l) => self.new_visible(&l),
                    None => self.new_hidden("skip"),
                };
                self.pt(initial_place, t);
                self.tp(t, final_place);
            }
            Powl::Xor(children) => {
                for c in children {
                    self.add_powl(
                        c,
                        Entity::Place(initial_place),
                        Some(Entity::Place(final_place)),
                    )?;
                }
            }
            Powl::Loop(children) => {
                let [do_part, redo] = &**children;
                let loop_start = self.new_place();
                let init_loop = self.new_hidden("init_loop");
                self.pt(initial_place, init_loop);
                self.tp(init_loop, loop_start);
                let back = self.new_hidden("loop");
                let after_do = self.add_powl(do_part, Entity::Place(loop_start), None)?;
                let after_redo = self.add_powl(redo, Entity::Place(after_do), None)?;
                self.add_powl(
                    &Powl::Silent,
                    Entity::Place(after_do),
                    Some(Entity::Place(final_place)),
                )?;
                self.pt(after_redo, back);
                self.tp(back, loop_start);
            }
            Powl::PartialOrder(po) => {
                let reduction = po.order().transitive_reduction()?;
                let split = self.new_hidden("tauSplit");
                self.pt(initial_place, split);
                let join = self.new_hidden("tauJoin");
                self.tp(join, final_place);
                let starts = reduction.start_nodes();
                let ends = reduction.end_nodes();
                let mut init_trans = Vec::new();
                let mut final_trans = Vec::new();
                for (i, c) in po.children().iter().enumerate() {
                    let i_trans = self.new_hidden("init_par");
                    if starts.contains(&i) {
                        let p = self.new_place();
                        self.tp(split, p);
                        self.pt(p, i_trans);
                    }
                    let f_trans = self.new_hidden("final_par");
                    if ends.contains(&i) {
                        let p = self.new_place();
                        self.tp(f_trans, p);
                        self.pt(p, join);
                    }
                    self.add_powl(
                        c,
                        Entity::Transition(i_trans),
                        Some(Entity::Transition(f_trans)),
                    )?;
                    init_trans.push(i_trans);
                    final_trans.push(f_trans);
                }
                for (i, j) in reduction.edges() {
                    let p = self.new_place();
                    self.tp(final_trans[i], p);
                    self.pt(p, init_trans[j]);
                }
            }
        }
        Ok(final_place)
    }
}

impl Powl {
    /// Converts the model to an accepting Petri net (pm4py's
    /// `powl.converter.apply`).
    ///
    /// The net has a `source` place with the initial token and a `sink`
    /// place for the final marking. Places are `p_<n>` and silent
    /// transitions are named after their role (`tauSplit_3`, `init_par_5`,
    /// ...) as in pm4py. Visible transitions are named `t_1`, `t_2`, ...
    /// where pm4py uses random UUIDs, and carry the label of the activity or
    /// frequent transition. pm4py also stores a frequent transition's
    /// activity and flags as transition properties; this net has none. The
    /// net is named `powl_net`; pm4py names it `imdf_net_<timestamp>`.
    ///
    /// A partial order connects the children along pm4py's
    /// [`transitive_reduction`](crate::powl::BinaryRelation::transitive_reduction)
    /// of its order. A cyclic order converts, to a net that deadlocks, as in
    /// pm4py.
    ///
    /// # Errors
    ///
    /// [`PowlError::Reflexive`] when a partial order relates a child to
    /// itself.
    pub fn to_petri_net(&self) -> Result<AcceptingPetriNet, PowlError> {
        let (mut b, source, sink) = Builder::new("powl_net");
        let initial_place = b.new_place();
        let t = b.new_hidden("tau");
        b.pt(source, t);
        b.tp(t, initial_place);
        let final_place = b.new_place();
        let t = b.new_hidden("tau");
        b.pt(final_place, t);
        b.tp(t, sink);
        b.add_powl(
            self,
            Entity::Place(initial_place),
            Some(Entity::Place(final_place)),
        )?;
        Ok(b.finish(source, sink))
    }
}
