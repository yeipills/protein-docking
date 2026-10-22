// Test script to verify API connection
import axios from 'axios';

async function testAPI() {
  const baseURL = 'http://localhost:3001/api';
  console.log('Testing API at:', baseURL);
  
  try {
    // Test health endpoint
    const healthResponse = await axios.get(`${baseURL}/health`);
    console.log('✅ Health check:', healthResponse.data);
    
    // Test jobs endpoint
    const jobsResponse = await axios.get(`${baseURL}/jobs`);
    console.log('✅ Jobs endpoint:', jobsResponse.data);
    
    // Create a test job
    const createJobResponse = await axios.post(`${baseURL}/jobs`, {
      receptor_file_id: 'test-receptor',
      ligand_file_id: 'test-ligand',
      config: { max_poses: 100 }
    });
    console.log('✅ Create job:', createJobResponse.data);
    
    // Check jobs again
    const jobsAfterCreate = await axios.get(`${baseURL}/jobs`);
    console.log('✅ Jobs after create:', jobsAfterCreate.data);
    
  } catch (error) {
    console.error('❌ API test failed:', error.message);
    if (error.response) {
      console.error('Response data:', error.response.data);
      console.error('Response status:', error.response.status);
    }
  }
}

testAPI();