
use ray_tracer::scene::Scene;
use ray_tracer::camera::perspective::PerspectiveCamera;
use ray_tracer::camera::Camera;
use ray_tracer::constants::Axis;
use ray_tracer::tree::TreeType;
use ray_tracer::vector::Vec3;
use ray_tracer::hitable::Hitable;
use ray_tracer::hitable::transform::Translation;
use ray_tracer::hitable::primitive::Sphere;
use ray_tracer::hitable::primitive::Cube;
use ray_tracer::hitable::primitive::Rectangle;
use ray_tracer::texture::uniform::UniformTexture;
use ray_tracer::texture::checker::CheckerTexture;
use ray_tracer::material::Material;
use ray_tracer::material::plain::PlainMaterial;
use ray_tracer::material::lambertian::LambertianMaterial;
use ray_tracer::material::metal::MetalMaterial;
use ray_tracer::material::dielectric::DielectricMaterial;
use ray_tracer::actor::Actor;
pub struct RayScene {
}

impl RayScene {
    pub fn load(_filename: &str) -> (Scene<f64>, PerspectiveCamera<f64>) {
        // TODO: actually load a scene...

        // This example is taken from
        // https://github.com/alesgenova/ray-tracer, by Alessandro Genova
        // test: random_scene()

        let mut scene = Scene::<f64>::new();
        scene.set_background(Vec3::from_array([0.5, 0.7, 0.9]));

        const N_SPHERES_X : usize = 20;
        const N_SPHERES_Y : usize = N_SPHERES_X;

        const MIN_X : f64 = -20.0;
        const MAX_X : f64 = 20.0;

        const MIN_Y : f64 = MIN_X;
        const MAX_Y : f64 = MAX_X;

        const MIN_RADIUS : f64 = 0.2;
        const MAX_RADIUS : f64 = 0.4;

        // const SPHERE_PROBABILITY : f64 = 0.66666666;

        // const LAMBERTIAN_PROBABILITY : f64 = 0.3333;
        // const METAL_PROBABILITY : f64 = 0.3333;

        const MIN_FUZZINESS : f64 = 0.0;
        const MAX_FUZZINESS : f64 = 0.4;

        const MIN_REFRACTIVE : f64 = 1.2;
        const MAX_REFRACTIVE : f64 = 2.4;

        for i in 0..N_SPHERES_X {
            for j in 0..N_SPHERES_Y {
                let radius = MIN_RADIUS + (MAX_RADIUS - MIN_RADIUS) * 0.5;
                let mut x = i as f64 + 0.5 * (1.0 - radius);
                x = MIN_X + (MAX_X - MIN_X) * x / N_SPHERES_X as f64;
                let mut y = j as f64 + 0.5 * (1.0 - radius);
                y = MIN_Y + (MAX_Y - MIN_Y) * y / N_SPHERES_Y as f64;

                let hitable : Box<dyn Hitable<f64>> = if i < (N_SPHERES_X / 2) {
                    let hitable = Box::new(Sphere::<f64>::new(radius));
                    Box::new(Translation::new(hitable, Vec3::from_array([x, y, radius])))
                } else {
                    let l = radius * 2.0 * 0.8;
                    let hitable = Box::new(Cube::<f64>::new(l, l, l));
                    Box::new(Translation::new(hitable, Vec3::from_array([x, y, radius * 0.8])))
                };

                let color = Vec3::from_array([(i as f64) / (N_SPHERES_X as f64), (j as f64) / (N_SPHERES_Y as f64), 0.5]);
                let texture = Box::new(UniformTexture::new(color));
                let material : Box<dyn Material<f64>> = if j < (N_SPHERES_Y / 2) {
                    Box::new(LambertianMaterial::<f64>::new(texture, 0.5))
                } else if i < (N_SPHERES_X / 2) {
                    let fuzziness = MIN_FUZZINESS + (MAX_FUZZINESS - MIN_FUZZINESS) * 0.5;
                    Box::new(MetalMaterial::<f64>::new(texture, fuzziness))
                } else {
                    let n = MIN_REFRACTIVE + (MAX_REFRACTIVE - MIN_REFRACTIVE) * 0.5;
                    Box::new(DielectricMaterial::<f64>::new(texture, n))
                };
                let actor = Actor::<f64> { hitable, material};
                scene.add_actor(actor);
            }
        }

        // Three larger spheres in the center
        let radius = 2.0;
        let sphere = Box::new(Sphere::<f64>::new(radius));
        let sphere = Translation::new(sphere, Vec3::from_array([0.0, 0.0, radius]));
        let color = Vec3::from_array([0.78, 1.0, 0.78]);
        let texture = Box::new(UniformTexture::new(color));
        let material = DielectricMaterial::<f64>::new(texture, 2.4);
        let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
        scene.add_actor(actor);

        let sphere = Box::new(Sphere::<f64>::new(radius));
        let sphere = Translation::new(sphere, Vec3::from_array([0.0, - 2.0 * radius, radius]));
        let color = Vec3::from_array([0.9, 0.9, 0.9]);
        let texture = Box::new(UniformTexture::new(color));
        let material = MetalMaterial::<f64>::new(texture, 0.0);
        let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
        scene.add_actor(actor);

        let sphere = Box::new(Sphere::<f64>::new(radius));
        let sphere = Translation::new(sphere, Vec3::from_array([0.0, 2.0 * radius, radius]));
        let color = Vec3::from_array([1.0, 0.15, 0.15]);
        let texture = Box::new(UniformTexture::new(color));
        let material = MetalMaterial::<f64>::new(texture, 0.1);
        let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
        scene.add_actor(actor);

        // Sphere used as light
        let radius = 4.0;
        let sphere = Box::new(Sphere::<f64>::new(radius));
        let sphere = Translation::new(sphere, Vec3::from_array([0.0, 1.0, 12.5]));
        let color = Vec3::from_array([1.0, 1.0, 1.0]);
        let texture = Box::new(UniformTexture::new(color));
        let material = PlainMaterial::<f64>::new(texture);
        let actor = Actor::<f64> { hitable: Box::new(sphere), material: Box::new(material)};
        scene.add_actor(actor);

        // Rectangle used as floor
        let length = 2000.0;
        let color0 = Vec3::from_array([1.0, 1.0, 1.0]);
        let color1 = Vec3::from_array([0.8, 0.8, 0.8]);
        let texture0 = UniformTexture::new(color0);
        let texture1 = UniformTexture::new(color1);
        let texture = Box::new(CheckerTexture::new(Box::new(texture0), Box::new(texture1)));

        let hitable = Box::new(Rectangle::<f64>::new(length, Axis::X, length, Axis::Y));
        let hitable = Box::new(Translation::new(hitable, Vec3::from_array([0.0, 0.0, -radius])));
        let material = Box::new(LambertianMaterial::<f64>::new(texture, 0.75));
        let actor = Actor::<f64> { hitable, material };
        scene.add_actor(actor);

        let mul = 4;
        let width = 16 * mul;
        let height = 9 * mul;
        let aspect = width as f64 / height as f64;
        let mut camera = PerspectiveCamera::<f64>::new();
        camera.set_aspect(aspect);
        camera.set_fov(0.25 * std::f64::consts::PI);
        camera.set_position(&[-6.0, -10.0, 3.0]);
        camera.set_lookat(&[0.0, 0.0, 2.0]);
        camera.set_up(&[0.0, 0.0, 1.0]);
        camera.set_aperture(0.0);
        let focus = (camera.get_lookat() - camera.get_position()).norm();
        camera.set_focus(focus);

        scene.set_tree_type(TreeType::Oct);

        (scene, camera)
    }
}
