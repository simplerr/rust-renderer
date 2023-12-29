/*
Terms
p_hat : target function/PDF which is hard to draw samples from
p     : proposal PDF which looks a little like the target PDF and drawing samples from it is easy
Xi    : candidate sample from the proposal PDF
m_i   : MIS weight, "If all Xi are identically distributed, use m_i = 1/M." [1]
W_Xi  : unbiased contribution weight for Xi. "If Xi has a known PDF p(Xi) use W_Xi = 1/p(Xi)." [1]
w_i   : resampling weight
      w_i = m_i(Xi) * p_hat(Xi) * W_Xi
w_sum : sum of all w_i
W_X   : unbiased contribution weight for the selected sample
      W_X = (1 / p_hat(X)) * w_sum

In the light sampling case, p_hat could be the contribution of a light source and p is simply a
uniform distribution over all light sources i.e 1/N. A way to think about it is that it is very
easy pick pick light samples uniformly but what we really want is to pick them according to their
contribution so we approximate that PDF by using RIS.

What is the target function in the light sampling case?
   "The integrand for direct illumination (with unspecified pixel index) is
   f(x) = fs(x)G(x)V(x)Le(x), and we recommend starting with the same target function, p_hat = f:
   p_hat(x) = f(x) = fs(x)G(x)V(x)Le(x)" [1]
   It's also possible to drop the visibility term as an optimization

Explanation about that p(x) does not need to be a PDF but rather a weight:
   "Let's think-what's the role of 1/p(X) in the f(X)/p(X) estimator? It's a
   weight for the sample f(X). Is this weight needed? Yes, absolutely. Does the
   weight need to be a PDF? Not exactly. What? Well, you see, RIS provides
   the sample X a weight, which we denote Wx. This weight produces an
   unbiased contribution f(X)Wx that estimates the integral of f. Weights
   are needed, but they need not be PDFs." [1]

How are two reservoirs combined?
   "To combine two reservoirs, we treat each reservoirs y as a fresh sample with weight wsum,
   and feed it as input to a new reservoir. The result is mathematically equivalent to having
   performed reservoir sampling on the two reservoirs combined input streams." [2]

   "To account for the fact that samples from the neighboring pixel q' are resampled following
   a different target distribution p_hat(q'), we reweight the samples with the factor p_hat_q(r.y) / p_hat_q'(r.y)
   to account for areas that were over- or undersampled at the neighbor compared to the current pixel.
   The resulting term p_hat_q(r.y) / p_hat_q'(r.y) * r.W_sum can be written more succinctly as
   p_hat_q(r.y) * r.W * r.M using the term already computed in Alg. 3, line 8" [2]

   References:
   [1] https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf
   [2] https://benedikt-bitterli.me/restir/bitterli20restir.pdf
*/

#define RIS_CANDIDATES_LIGHTS 32

struct Reservoir
{
   int Y;         // index of most important light
   float W_sum;   // sum of weights
   float W_X;     // unbiased contribution weight
   int M;         // number of samples
};

vec3 get_light_intensity(in Light light, float distance_to_light)
{
   return light.intensity / pow(distance_to_light, 2.0);
}

float target_function(int light_index, in vec3 hit_position)
{
   Light light = lightsSSBO.lights[light_index];
   float distance_to_light = distance(light.pos, hit_position);
   return luminance(get_light_intensity(light, distance_to_light));
}

void sample_light_uniform(inout uint rngState, out int selected_light_index, out float light_sample_weight)
{
   uint num_used_lights = view.num_lights;
   num_used_lights = min(num_used_lights, view.max_num_lights_used);
   selected_light_index = int(randomFloat(rngState) * float(num_used_lights));
   light_sample_weight = 1.0 / float(num_used_lights);
}

void finalize_resampling(inout Reservoir reservoir, float p_hat)
{
   reservoir.W_X = (p_hat == 0.0) ? 0.0 : (1.0 / p_hat) * reservoir.W_sum / reservoir.M;
}

// Together with the resample() function this is Algorithm 2 from [1]
void updateReservoir(inout uint rngState, inout Reservoir reservoir, int Xi, float w_i, int M)
{
   reservoir.W_sum += w_i;
   reservoir.M += M;

   // random < (w_i / W_sum) in the papers but this avoids divide by 0
   if (randomFloat(rngState) * reservoir.W_sum < w_i) {
      reservoir.Y = Xi;
   }
}

Reservoir resample(inout uint rngState, in vec3 hit_position)
{
   Reservoir reservoir;
   reservoir.Y = -1;
   reservoir.W_sum = 0.0;
   reservoir.W_X = 0.0;
   reservoir.M = 0;

   const int M = RIS_CANDIDATES_LIGHTS;

   for (int i = 0; i < M; i++) {
      // Generate candidate sample (Xi)
      int candidate_index;
      float p = 0.0;
      sample_light_uniform(rngState, candidate_index, p);

      // Calculate resampling weight (w_i)
      float m_i = 1.0 / float(M); // MIS weight
      float p_hat = target_function(candidate_index, hit_position);
      float W_Xi = 1.0 / p;
      float w_i = m_i * p_hat * W_Xi;

      updateReservoir(rngState, reservoir, candidate_index, w_i, 1);
   }

   // Does not really matter, will be the same for all reservoirs anyways
   reservoir.M = 1;

   // Todo: need to check so we don't have -1 as index when the sample is used
   if (reservoir.Y != -1) {
      float p_hat = target_function(reservoir.Y, hit_position);
      finalize_resampling(reservoir, p_hat);
   }

   return reservoir;
}

