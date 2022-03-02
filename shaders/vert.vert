#version 460

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec3 inUv;

layout(location = 0) out vec3 outWorldPos;
layout(location = 1) out vec3 outNormal;
layout(location = 2) out vec3 outUv;


layout(set = 0, binding = 0) uniform view_proj_matrices {
	mat4 vp;
} view_proj;

layout(set = 0, binding = 1) uniform model_matrix {
	mat4 model;
} model;


// layout( push_constant ) uniform matrix {
// 	mat4 model;
// } PushConstant;

void main()
{
	vec4 model_position = model.model * vec4(inPosition, 1.0);

	vec4 final_position = view_proj.vp * model_position;


	outWorldPos = vec3(model_position);
	outNormal = mat3(model.model) * inNormal;
	// outNormal = inNormal;
	outUv = inUv;

	gl_Position = final_position;
}