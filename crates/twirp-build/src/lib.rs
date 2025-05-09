use std::fmt::Write;

/// Generates twirp services for protobuf rpc service definitions.
///
/// In your `build.rs`, using `prost_build`, you can wire in the twirp
/// `ServiceGenerator` to produce a Rust server for your proto services.
///
/// Add a call to `.service_generator(twirp_build::service_generator())` in
/// main() of `build.rs`.
pub fn service_generator() -> Box<ServiceGenerator> {
    Box::new(ServiceGenerator {})
}

pub struct ServiceGenerator;

impl prost_build::ServiceGenerator for ServiceGenerator {
    fn generate(&mut self, service: prost_build::Service, buf: &mut String) {
        let service_name = service.name;
        let service_fqn = format!("{}.{}", service.package, service.proto_name);
        writeln!(buf).unwrap();

        //
        // generate the twirp server
        //
        writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        writeln!(buf, "pub trait {} {{", service_name).unwrap();
        writeln!(buf, "    type Error;").unwrap();
        for m in &service.methods {
            writeln!(
                buf,
                "    async fn {}(&self, ctx: twirp::Context, req: {}) -> Result<{}, Self::Error>;",
                m.name, m.input_type, m.output_type,
            )
            .unwrap();
        }
        writeln!(buf, "}}").unwrap();

        writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        writeln!(buf, "impl<T> {service_name} for std::sync::Arc<T>").unwrap();
        writeln!(buf, "where").unwrap();
        writeln!(buf, "    T: {service_name} + Sync + Send").unwrap();
        writeln!(buf, "{{").unwrap();
        writeln!(buf, "    type Error = T::Error;\n").unwrap();
        for m in &service.methods {
            writeln!(
                buf,
                "    async fn {}(&self, ctx: twirp::Context, req: {}) -> Result<{}, Self::Error> {{",
                m.name, m.input_type, m.output_type,
            )
                .unwrap();
            writeln!(buf, "        T::{}(&*self, ctx, req).await", m.name).unwrap();
            writeln!(buf, "    }}").unwrap();
        }
        writeln!(buf, "}}").unwrap();

        //
        // generate the twirp client
        //
        writeln!(buf).unwrap();
        writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        writeln!(buf, "pub trait {service_name}Client: Send + Sync {{",).unwrap();
        for m in &service.methods {
            // Define: <METHOD>
            writeln!(
                buf,
                "    async fn {}(&self, req: {}) -> Result<{}, twirp::ClientError>;",
                m.name, m.input_type, m.output_type,
            )
            .unwrap();
        }
        writeln!(buf, "}}").unwrap();

        // Implement the rpc traits for: `twirp::client::Client`
        writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        writeln!(
            buf,
            "impl {service_name}Client for twirp::client::Client {{",
        )
        .unwrap();
        for m in &service.methods {
            // Define the rpc `<METHOD>`
            writeln!(
                buf,
                "    async fn {}(&self, req: {}) -> Result<{}, twirp::ClientError> {{",
                m.name, m.input_type, m.output_type,
            )
            .unwrap();
            writeln!(
                buf,
                r#"    self.request("{}/{}", req).await"#,
                service_fqn, m.proto_name
            )
            .unwrap();
            writeln!(buf, "    }}").unwrap();
        }
        writeln!(buf, "}}").unwrap();
    }
}
