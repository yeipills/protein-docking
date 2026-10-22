/*!
 * Test Worker - Worker básico para verificar funcionalidad
 */

// Worker simple para verificar que el sistema de workers funciona
self.onmessage = function(e) {
  if (e.data === 'test') {
    // Realizar test básico
    const start = performance.now();
    
    // Operación matemática simple
    let sum = 0;
    for (let i = 0; i < 100000; i++) {
      sum += Math.sqrt(i);
    }
    
    const duration = performance.now() - start;
    
    // Responder con éxito
    self.postMessage('test-ok');
  }
};

// Export para TypeScript
export {};