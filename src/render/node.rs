use gltf::{iter::Nodes as GltfNodes, scene::Transform, Scene};
use glam::{Mat4, Quat, Vec3};

fn compute_transform_matrix(transform: &Transform) -> Mat4 {
    match transform {
        Transform::Matrix { matrix } => Mat4::from_cols_array_2d(matrix),
        Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => {
            let translation = Mat4::from_translation(Vec3::from(*translation));
            let rotation = Mat4::from_quat(Quat::from_array(*rotation));
            let scale = Mat4::from_scale(Vec3::from(*scale));
            translation * rotation * scale
        }
    }
}

fn transform_2_scale_rot_position(transform: &Transform) -> (Vec3, Quat, Vec3) {
    match transform {
        Transform::Matrix { matrix } => {
            let m = Mat4::from_cols_array_2d(matrix);
            m.to_scale_rotation_translation()
        }
        Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => {
            let translation = Vec3::from(*translation);
            let rotation = Quat::from_array(*rotation);
            let scale = Vec3::from(*scale);
            (scale, rotation, translation)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Nodes {
    nodes: Vec<Node>,
    roots_indices: Vec<usize>,
    depth_first_taversal_indices: Vec<(usize, Option<usize>)>,
}

impl Nodes {
    pub fn from_gltf(gltf_nodes: GltfNodes, scene: &Scene) -> Nodes {
        let roots_indices = scene.nodes().map(|n| n.index()).collect::<Vec<_>>();
        let node_count = gltf_nodes.len();
        let mut nodes = Vec::with_capacity(node_count);
        for node in gltf_nodes {
            let node_index = node.index();
            let local_transform = node.transform();
            let global_transform_matrix = compute_transform_matrix(&local_transform);
            let mesh_index = node.mesh().map(|m| m.index());
            let skin_index = node.skin().map(|s| s.index());
            let light_index = node.light().map(|l| l.index());
            let children_indices = node.children().map(|c| c.index()).collect::<Vec<_>>();

            let (local_scale, local_rotation, local_position) = transform_2_scale_rot_position(&local_transform);
            let node = Node {
                local_position,
                local_rotation,
                local_scale,
                global_transform_matrix,
                mesh_index,
                skin_index,
                light_index,
                children_indices,
            };
            nodes.insert(node_index, node);
        }

        let mut nodes = Nodes::new(nodes, roots_indices);
        nodes.transform(None);
        nodes
    }

    fn new(nodes: Vec<Node>, roots_indices: Vec<usize>) -> Self {
        let depth_first_taversal_indices = build_graph_run_indices(&roots_indices, &nodes);
        Self {
            nodes,
            roots_indices,
            depth_first_taversal_indices,
        }
    }
}

impl Nodes {
    pub fn transform(&mut self, global_transform: Option<Mat4>) {
        for (index, parent_index) in &self.depth_first_taversal_indices {
            let parent_transform = parent_index
                .map(|id| {
                    let parent = &self.nodes[id];
                    parent.global_transform_matrix
                })
                .or(global_transform);

            if let Some(matrix) = parent_transform {
                let node = &mut self.nodes[*index];
                node.apply_transform(matrix);
            }
        }
    }

    pub fn get_skins_transform(&self) -> Vec<(usize, Mat4)> {
        self.nodes
            .iter()
            .filter(|n| n.skin_index.is_some())
            .map(|n| (n.skin_index.unwrap(), n.transform()))
            .collect::<Vec<_>>()
    }
}

fn build_graph_run_indices(roots_indices: &[usize], nodes: &[Node]) -> Vec<(usize, Option<usize>)> {
    let mut indices = Vec::new();
    for root_index in roots_indices {
        build_graph_run_indices_rec(nodes, *root_index, None, &mut indices);
    }
    indices
}

fn build_graph_run_indices_rec(
    nodes: &[Node],
    node_index: usize,
    parent_index: Option<usize>,
    indices: &mut Vec<(usize, Option<usize>)>,
) {
    indices.push((node_index, parent_index));
    for child_index in &nodes[node_index].children_indices {
        build_graph_run_indices_rec(nodes, *child_index, Some(node_index), indices);
    }
}

impl Nodes {
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut [Node] {
        &mut self.nodes
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    local_position: Vec3,
    local_scale: Vec3,
    local_rotation: Quat,
    global_transform_matrix: Mat4,
    mesh_index: Option<usize>,
    skin_index: Option<usize>,
    light_index: Option<usize>,
    children_indices: Vec<usize>,
}

impl Node {
    fn get_local_transform(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.local_scale, self.local_rotation, self.local_position)
    }

    fn apply_transform(&mut self, transform: Mat4) {
        self.global_transform_matrix = transform * self.get_local_transform();
    }
}

impl Node {
    pub fn transform(&self) -> Mat4 {
        self.global_transform_matrix
    }

    pub fn mesh_index(&self) -> Option<usize> {
        self.mesh_index
    }

    pub fn skin_index(&self) -> Option<usize> {
        self.skin_index
    }

    pub fn light_index(&self) -> Option<usize> {
        self.light_index
    }

    pub fn set_local_position(&mut self, translation: Vec3) {
        self.local_position = translation;
    }

    pub fn set_rotation(&mut self, rotation: Quat) {
        self.local_rotation = rotation;
    }

    pub fn set_local_scale(&mut self, scale: Vec3) {
        self.local_scale = scale;
    }
}

