
use ray_tracer::scene::Scene;
use ray_tracer::camera::perspective::PerspectiveCamera;


pub struct RayScene {

}

impl RayScene {
    pub fn load(filename: &str) -> (Scene<f64>, PerspectiveCamera<f64>) {
        let mut scene = Scene::<f64>::new();
        let mut camera = PerspectiveCamera::<f64>::new();

        (scene, camera)
    }
}
