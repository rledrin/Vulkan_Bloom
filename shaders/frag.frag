#version 460

layout(location = 0) in vec3 inWorldPos;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec3 inUv;


layout(location = 0) out vec4 outColor;


struct Light {
	vec3 light_position;
	vec3 light_color;
};

layout(set = 1, binding = 0) uniform PbrParameters{
	vec3 albedo;
	float metallic;
	float roughness;
	float ao;
	vec3 cam_pos;
	Light lights[1];
} parameters;


const float PI = 3.14159265359;

float DistributionGGX(vec3 N, vec3 H, float roughness)
{
	float a = roughness*roughness;
	float a2 = a*a;
	float NdotH = max(dot(N, H), 0.0);
	float NdotH2 = NdotH*NdotH;

	float nom = a2;
	float denom = (NdotH2 * (a2 - 1.0) + 1.0);
	denom = PI * denom * denom;

	return nom / denom;
}
// ----------------------------------------------------------------------------
float GeometrySchlickGGX(float NdotV, float roughness)
{
	float r = (roughness + 1.0);
	float k = (r*r) / 8.0;

	float nom = NdotV;
	float denom = NdotV * (1.0 - k) + k;

	return nom / denom;
}
// ----------------------------------------------------------------------------
float GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness)
{
	float NdotV = max(dot(N, V), 0.0);
	float NdotL = max(dot(N, L), 0.0);
	float ggx2 = GeometrySchlickGGX(NdotV, roughness);
	float ggx1 = GeometrySchlickGGX(NdotL, roughness);

	return ggx1 * ggx2;
}
// ----------------------------------------------------------------------------
vec3 fresnelSchlick(float cosTheta, vec3 F0)
{
	return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}
// ----------------------------------------------------------------------------


// https://github.com/dmnsgn/glsl-tone-map/blob/master/aces.glsl
vec3 aces(vec3 x) {
	const float a = 2.51;
	const float b = 0.03;
	const float c = 2.43;
	const float d = 0.59;
	const float e = 0.14;
	return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

// can be optimized into lut (compute can gen it)
float GTTonemap(float x) {
	float m = 0.22; // linear section start
	float a = 1.0;  // contrast
	float c = 1.33; // black brightness
	float P = 1.0;  // maximum brightness
	float l = 0.4;  // linear section length
	float l0 = ((P-m)*l) / a; // 0.312
	float S0 = m + l0; // 0.532
	float S1 = m + a * l0; // 0.532
	float C2 = (a*P) / (P - S1); // 2.13675213675
	float L = m + a * (x - m);
	float T = m * pow(x/m, c);
	float S = P - (P - S1) * exp(-C2*(x - S0)/P);
	float w0 = 1 - smoothstep(0.0, m, x);
	float w2 = (x < m+l)?0:1;
	float w1 = 1 - w0 - w2;
	return float(T * w0 + L * w1 + S * w2);
}

// this costs about 0.2-0.3ms more than aces, as-is
vec3 GTTonemap(vec3 x) {
	return vec3(
		GTTonemap(x.r),
		GTTonemap(x.g),
		GTTonemap(x.b)
	);
}

void main()
{
	vec3 N = normalize(inNormal);
	vec3 V = normalize(parameters.cam_pos - inWorldPos);

	vec3 F0 = vec3(0.04); 
	F0 = mix(F0, parameters.albedo, parameters.metallic);

	// reflectance equation
	vec3 Lo = vec3(0.0);
	for(int i = 0; i < 1; ++i) 
	{
		// calculate per-light radiance
		vec3 L = normalize(parameters.lights[i].light_position - inWorldPos);
		vec3 H = normalize(V + L);
		float distance = length(parameters.lights[i].light_position - inWorldPos);
		float attenuation = 1.0 / (distance * distance);
		vec3 radiance = parameters.lights[i].light_color * attenuation;

		// cook-torrance brdf
		float NDF = DistributionGGX(N, H, parameters.roughness);
		float G = GeometrySmith(N, V, L, parameters.roughness);
		vec3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);

		vec3 kS = F;
		vec3 kD = vec3(1.0) - kS;
		kD *= 1.0 - parameters.metallic;

		vec3 numerator = NDF * G * F;
		float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
		vec3 specular = numerator / denominator;

		// add to outgoing radiance Lo
		float NdotL = max(dot(N, L), 0.0);
		Lo += (kD * parameters.albedo / PI + specular) * radiance * NdotL; 
	}

	vec3 ambient = vec3(0.03) * parameters.albedo * parameters.ao;
	vec3 color = ambient + Lo;

	// color = color / (color + vec3(1.0));
	// color = pow(color, vec3(1.0/2.2));
	// vec3 tone_mapped_color = color;

	// vec3 tone_mapped_color = color * 3;

	// // vec3 tone_mapped_color = aces(color);
	// vec3 tone_mapped_color = GTTonemap(color);
	// // tone_mapped_color = pow(tone_mapped_color, vec3(1.0/2.2));

	// outColor = vec4(color, 1.0);
	outColor = vec4(5.5, 4.62, 1.014, 1.0);
}